import type { IdentityContent, ItemContentOf, PaymentCardContent, RevealableField } from "../../domain/vaultTypes";

/** Descripción declarativa de un campo de texto: sirve a la vez para el editor y para la vista de detalle. */
export interface ContentFieldDescriptor<Content> {
  fieldKey: keyof Content & string;
  labelKey: string;
  revealableField?: RevealableField;
  inputMode?: "text" | "numeric" | "email" | "tel";
  gridColumnSpan?: 1 | 2;
}

export const PAYMENT_CARD_FIELD_DESCRIPTORS: ContentFieldDescriptor<PaymentCardContent>[] = [
  { fieldKey: "cardholderName", labelKey: "fields.cardholderName", gridColumnSpan: 2 },
  { fieldKey: "cardBrand", labelKey: "fields.cardBrand", gridColumnSpan: 2 },
  { fieldKey: "cardNumber", labelKey: "fields.cardNumber", revealableField: { fieldKind: "cardNumber" }, inputMode: "numeric", gridColumnSpan: 2 },
  { fieldKey: "expirationMonth", labelKey: "fields.expirationMonth", inputMode: "numeric" },
  { fieldKey: "expirationYear", labelKey: "fields.expirationYear", inputMode: "numeric" },
  { fieldKey: "securityCode", labelKey: "fields.securityCode", revealableField: { fieldKind: "cardSecurityCode" }, inputMode: "numeric" },
];

export const IDENTITY_FIELD_DESCRIPTORS: ContentFieldDescriptor<IdentityContent>[] = [
  { fieldKey: "honorificTitle", labelKey: "fields.honorificTitle" },
  { fieldKey: "firstName", labelKey: "fields.firstName" },
  { fieldKey: "lastName", labelKey: "fields.lastName", gridColumnSpan: 2 },
  { fieldKey: "emailAddress", labelKey: "fields.emailAddress", inputMode: "email" },
  { fieldKey: "phoneNumber", labelKey: "fields.phoneNumber", inputMode: "tel" },
  { fieldKey: "companyName", labelKey: "fields.companyName", gridColumnSpan: 2 },
  { fieldKey: "streetAddress", labelKey: "fields.streetAddress", gridColumnSpan: 2 },
  { fieldKey: "city", labelKey: "fields.city" },
  { fieldKey: "postalCode", labelKey: "fields.postalCode" },
  { fieldKey: "country", labelKey: "fields.country", gridColumnSpan: 2 },
  {
    fieldKey: "identityDocumentNumber",
    labelKey: "fields.identityDocumentNumber",
    revealableField: { fieldKind: "identityDocumentNumber" },
    gridColumnSpan: 2,
  },
];

export type DescriptorDrivenContent = ItemContentOf<"paymentCard"> | ItemContentOf<"identity">;
