export type AvatarCrop = {
  zoom: number;
  horizontal: number;
  vertical: number;
};

export type AvatarCropRect = {
  x: number;
  y: number;
  size: number;
};

export const DEFAULT_AVATAR_CROP: AvatarCrop = {
  zoom: 1,
  horizontal: 0,
  vertical: 0,
};

export const AVATAR_CROP_OUTPUT_SIZE = 512;

const MAX_AVATAR_BYTES = 5 * 1024 * 1024;
const MAX_AVATAR_ZOOM = 3;
const SUPPORTED_AVATAR_TYPES = new Set(["image/jpeg", "image/png", "image/webp"]);

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Number.isFinite(value) ? value : min));
}

export function calculateAvatarCrop(width: number, height: number, crop: AvatarCrop): AvatarCropRect {
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    throw new RangeError("头像图片尺寸无效。");
  }

  const size = Math.min(width, height) / clamp(crop.zoom, 1, MAX_AVATAR_ZOOM);
  const horizontalSpace = (width - size) / 2;
  const verticalSpace = (height - size) / 2;

  return {
    x: horizontalSpace * (1 + clamp(crop.horizontal, -1, 1)),
    y: verticalSpace * (1 + clamp(crop.vertical, -1, 1)),
    size,
  };
}

type DecodedAvatarImage = {
  source: CanvasImageSource;
  width: number;
  height: number;
  close: () => void;
};

async function decodeAvatarImage(file: File): Promise<DecodedAvatarImage> {
  if (typeof globalThis.createImageBitmap === "function") {
    const bitmap = await globalThis.createImageBitmap(file);
    return {
      source: bitmap,
      width: bitmap.width,
      height: bitmap.height,
      close: () => bitmap.close(),
    };
  }

  const objectUrl = URL.createObjectURL(file);
  const image = new Image();
  try {
    await new Promise<void>((resolve, reject) => {
      image.onload = () => resolve();
      image.onerror = () => reject(new Error("头像图片无法读取。"));
      image.src = objectUrl;
    });
    return {
      source: image,
      width: image.naturalWidth,
      height: image.naturalHeight,
      close: () => URL.revokeObjectURL(objectUrl),
    };
  } catch (error) {
    URL.revokeObjectURL(objectUrl);
    throw error;
  }
}

function canvasToPng(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error("头像图片处理失败，请重新选择图片。"));
    }, "image/png");
  });
}

export async function cropAvatarImage(file: File, crop: AvatarCrop): Promise<File> {
  if (!SUPPORTED_AVATAR_TYPES.has(file.type)) {
    throw new Error("头像格式无效，仅支持 JPEG、PNG 或 WebP。");
  }

  let image: DecodedAvatarImage;
  try {
    image = await decodeAvatarImage(file);
  } catch {
    throw new Error("头像图片无法读取，请选择有效的 JPEG、PNG 或 WebP 文件。");
  }

  try {
    const rect = calculateAvatarCrop(image.width, image.height, crop);
    const canvas = document.createElement("canvas");
    canvas.width = AVATAR_CROP_OUTPUT_SIZE;
    canvas.height = AVATAR_CROP_OUTPUT_SIZE;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("当前浏览器无法处理头像图片。");

    context.drawImage(
      image.source,
      rect.x,
      rect.y,
      rect.size,
      rect.size,
      0,
      0,
      AVATAR_CROP_OUTPUT_SIZE,
      AVATAR_CROP_OUTPUT_SIZE,
    );
    context.globalCompositeOperation = "destination-in";
    context.beginPath();
    context.arc(
      AVATAR_CROP_OUTPUT_SIZE / 2,
      AVATAR_CROP_OUTPUT_SIZE / 2,
      AVATAR_CROP_OUTPUT_SIZE / 2,
      0,
      Math.PI * 2,
    );
    context.fill();

    const blob = await canvasToPng(canvas);
    if (blob.size > MAX_AVATAR_BYTES) {
      throw new Error("裁切后的头像仍超过 5 MiB，请选择尺寸更小的图片。");
    }
    return new File([blob], "avatar.png", { type: "image/png" });
  } finally {
    image.close();
  }
}
