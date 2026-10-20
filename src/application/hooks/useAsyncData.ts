import { useCallback, useEffect, useState } from "react";

import type { CommandError } from "../../domain/vaultTypes";

export interface AsyncDataState<Data> {
  loadedData: Data | null;
  isLoading: boolean;
  loadError: CommandError | null;
  reload: () => void;
}

/** Carga datos del backend y descarta respuestas obsoletas si las dependencias cambian. */
export function useAsyncData<Data>(loadData: () => Promise<Data>, dependencies: readonly unknown[]): AsyncDataState<Data> {
  const [loadedData, setLoadedData] = useState<Data | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<CommandError | null>(null);
  const [reloadCounter, setReloadCounter] = useState(0);

  const stableLoadData = useCallback(loadData, dependencies);

  useEffect(() => {
    let isStillRelevant = true;
    setIsLoading(true);
    stableLoadData()
      .then((freshData) => {
        if (!isStillRelevant) return;
        setLoadedData(freshData);
        setLoadError(null);
      })
      .catch((commandError: CommandError) => {
        if (!isStillRelevant) return;
        setLoadError(commandError);
      })
      .finally(() => {
        if (isStillRelevant) setIsLoading(false);
      });
    return () => {
      isStillRelevant = false;
    };
  }, [stableLoadData, reloadCounter]);

  const reload = useCallback(() => setReloadCounter((currentCounter) => currentCounter + 1), []);
  return { loadedData, isLoading, loadError, reload };
}
