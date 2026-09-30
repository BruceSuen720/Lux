import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef } from "react";
import { queryKeys } from "../../lib/api/query-keys";

type UserEventScope = "home";

export function useUserEvents() {
  const queryClient = useQueryClient();
  const homeRefreshPending = useRef(false);
  const refreshHome = useCallback(() => {
    const homeQuery = queryClient.getQueryCache().find({ queryKey: queryKeys.home });
    if (
      !homeQuery
      || (homeQuery.state.error && homeQuery.state.data === undefined)
    ) {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.home,
        refetchType: "none",
      });
      return;
    }

    if (homeQuery.state.fetchStatus === "fetching") {
      if (homeRefreshPending.current) return;
      homeRefreshPending.current = true;
      void queryClient
        .invalidateQueries(
          { queryKey: queryKeys.home },
          { cancelRefetch: false },
        )
        .finally(() => {
          homeRefreshPending.current = false;
          const currentHomeQuery = queryClient
            .getQueryCache()
            .find({ queryKey: queryKeys.home });
          if (currentHomeQuery?.state.data !== undefined) {
            void queryClient.invalidateQueries({ queryKey: queryKeys.home });
          }
        });
      return;
    }

    void queryClient.invalidateQueries(
      { queryKey: queryKeys.home },
      { cancelRefetch: false },
    );
  }, [queryClient]);

  useEffect(() => {
    if (typeof EventSource === "undefined") return undefined;

    const source = new EventSource("/api/v1/events");
    const invalidate = (scope: UserEventScope) => {
      if (scope !== "home") return;
      refreshHome();
      void queryClient.invalidateQueries({ queryKey: queryKeys.libraries });
      void queryClient.invalidateQueries({ queryKey: ["library"] });
    };
    const handleOpen = () => invalidate("home");
    const handleInvalidate = (event: Event) => {
      const scope = parseScope((event as MessageEvent<string>).data);
      if (scope) invalidate(scope);
    };

    source.addEventListener("open", handleOpen);
    source.addEventListener("invalidate", handleInvalidate);
    return () => {
      source.removeEventListener("open", handleOpen);
      source.removeEventListener("invalidate", handleInvalidate);
      source.close();
    };
  }, [queryClient, refreshHome]);
}

function parseScope(data: string): UserEventScope | null {
  try {
    const payload: unknown = JSON.parse(data);
    if (!payload || typeof payload !== "object" || !("scope" in payload)) return null;
    return payload.scope === "home" ? "home" : null;
  } catch {
    return null;
  }
}
