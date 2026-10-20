import { useEffect, useState } from "react";

export function useDebouncedValue<Value>(latestValue: Value, delayMilliseconds: number): Value {
  const [debouncedValue, setDebouncedValue] = useState(latestValue);
  useEffect(() => {
    const timeoutId = window.setTimeout(() => setDebouncedValue(latestValue), delayMilliseconds);
    return () => window.clearTimeout(timeoutId);
  }, [latestValue, delayMilliseconds]);
  return debouncedValue;
}
