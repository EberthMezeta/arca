export function Spinner({ sizeClassName = "size-4" }: { sizeClassName?: string }) {
  return (
    <span
      aria-hidden="true"
      className={`inline-block animate-spin rounded-full border-2 border-current border-r-transparent ${sizeClassName}`}
    />
  );
}
