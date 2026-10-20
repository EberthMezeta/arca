import { useTranslation } from "react-i18next";

import type { ItemId, LoginContent } from "../../domain/vaultTypes";
import { PlainDetailRow, SecretDetailRow, TotpDetailRow } from "../components/DetailRows";

export function LoginContentDetails({ itemId, itemContent }: { itemId: ItemId; itemContent: LoginContent }) {
  const { t: translate } = useTranslation();
  return (
    <>
      {itemContent.username && (
        <PlainDetailRow
          itemId={itemId}
          rowLabel={translate("fields.username")}
          displayedValue={itemContent.username}
          copyableField={{ fieldKind: "username" }}
        />
      )}
      {itemContent.password && (
        <SecretDetailRow
          itemId={itemId}
          rowLabel={translate("fields.password")}
          revealableField={{ fieldKind: "password" }}
          maskedValue={itemContent.password}
        />
      )}
      {itemContent.totpSecret && <TotpDetailRow itemId={itemId} />}
      {itemContent.websiteUrls.map((websiteUrl) => (
        <PlainDetailRow key={websiteUrl} itemId={itemId} rowLabel={translate("fields.website")} displayedValue={websiteUrl} />
      ))}
    </>
  );
}
