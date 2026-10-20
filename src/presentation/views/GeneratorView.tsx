import { useTranslation } from "react-i18next";

import { PasswordGeneratorPanel } from "../components/PasswordGeneratorPanel";

export function GeneratorView() {
  const { t: translate } = useTranslation();
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-xl flex-col gap-6 p-8">
        <header>
          <h1 className="text-2xl font-semibold">{translate("generator.title")}</h1>
          <p className="mt-1 text-sm text-muted">{translate("generator.subtitle")}</p>
        </header>
        <div className="rounded-2xl border border-border bg-surface p-6">
          <PasswordGeneratorPanel />
        </div>
      </div>
    </div>
  );
}
