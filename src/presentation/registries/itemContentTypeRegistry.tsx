import { CreditCard, IdCard, KeyRound, type LucideIcon, StickyNote } from "lucide-react";
import type { ComponentType } from "react";

import type { ItemContentOf, ItemContentType, ItemId } from "../../domain/vaultTypes";
import { IDENTITY_FIELD_DESCRIPTORS, PAYMENT_CARD_FIELD_DESCRIPTORS } from "../editors/contentFieldDescriptors";
import { DescriptorDrivenDetails } from "../editors/DescriptorDrivenDetails";
import { DescriptorDrivenEditor } from "../editors/DescriptorDrivenEditor";
import { LoginContentDetails } from "../editors/LoginContentDetails";
import { LoginContentEditor } from "../editors/LoginContentEditor";

export interface ContentEditorProps<ContentType extends ItemContentType> {
  itemContent: ItemContentOf<ContentType>;
  onContentChange: (itemContent: ItemContentOf<ContentType>) => void;
}

export interface ContentDetailsProps<ContentType extends ItemContentType> {
  itemId: ItemId;
  itemContent: ItemContentOf<ContentType>;
}

export interface ItemContentTypeDefinition<ContentType extends ItemContentType> {
  typeIcon: LucideIcon;
  singularLabelKey: string;
  pluralLabelKey: string;
  accentClassName: string;
  createEmptyContent: () => ItemContentOf<ContentType>;
  ContentEditor: ComponentType<ContentEditorProps<ContentType>> | null;
  ContentDetails: ComponentType<ContentDetailsProps<ContentType>> | null;
}

type ItemContentTypeRegistry = { [ContentType in ItemContentType]: ItemContentTypeDefinition<ContentType> };

/**
 * Fuente única de verdad por tipo de elemento: icono, textos, contenido vacío, editor y detalle.
 * Añadir un tipo = añadir una entrada (TypeScript avisa si falta alguna).
 */
export const ITEM_CONTENT_TYPE_REGISTRY: ItemContentTypeRegistry = {
  login: {
    typeIcon: KeyRound,
    singularLabelKey: "itemTypes.login",
    pluralLabelKey: "itemTypes.logins",
    accentClassName: "bg-accent-soft text-accent",
    createEmptyContent: () => ({ contentType: "login", username: "", password: "", websiteUrls: [""], totpSecret: null }),
    ContentEditor: LoginContentEditor,
    ContentDetails: LoginContentDetails,
  },
  secureNote: {
    typeIcon: StickyNote,
    singularLabelKey: "itemTypes.secureNote",
    pluralLabelKey: "itemTypes.secureNotes",
    accentClassName: "bg-warning-soft text-warning",
    createEmptyContent: () => ({ contentType: "secureNote" }),
    ContentEditor: null,
    ContentDetails: null,
  },
  paymentCard: {
    typeIcon: CreditCard,
    singularLabelKey: "itemTypes.paymentCard",
    pluralLabelKey: "itemTypes.paymentCards",
    accentClassName: "bg-success-soft text-success",
    createEmptyContent: () => ({
      contentType: "paymentCard",
      cardholderName: "",
      cardBrand: "",
      cardNumber: "",
      expirationMonth: "",
      expirationYear: "",
      securityCode: "",
    }),
    ContentEditor: ({ itemContent, onContentChange }) => (
      <DescriptorDrivenEditor itemContent={itemContent} fieldDescriptors={PAYMENT_CARD_FIELD_DESCRIPTORS} onContentChange={onContentChange} />
    ),
    ContentDetails: ({ itemId, itemContent }) => (
      <DescriptorDrivenDetails itemId={itemId} itemContent={itemContent} fieldDescriptors={PAYMENT_CARD_FIELD_DESCRIPTORS} />
    ),
  },
  identity: {
    typeIcon: IdCard,
    singularLabelKey: "itemTypes.identity",
    pluralLabelKey: "itemTypes.identities",
    accentClassName: "bg-danger-soft text-danger",
    createEmptyContent: () => ({
      contentType: "identity",
      honorificTitle: "",
      firstName: "",
      lastName: "",
      emailAddress: "",
      phoneNumber: "",
      streetAddress: "",
      city: "",
      postalCode: "",
      country: "",
      companyName: "",
      identityDocumentNumber: "",
    }),
    ContentEditor: ({ itemContent, onContentChange }) => (
      <DescriptorDrivenEditor itemContent={itemContent} fieldDescriptors={IDENTITY_FIELD_DESCRIPTORS} onContentChange={onContentChange} />
    ),
    ContentDetails: ({ itemId, itemContent }) => (
      <DescriptorDrivenDetails itemId={itemId} itemContent={itemContent} fieldDescriptors={IDENTITY_FIELD_DESCRIPTORS} />
    ),
  },
};

export const ITEM_CONTENT_TYPES = Object.keys(ITEM_CONTENT_TYPE_REGISTRY) as ItemContentType[];

/** Acceso tipado a la definición de un tipo concreto. */
export function definitionForContentType<ContentType extends ItemContentType>(
  contentType: ContentType,
): ItemContentTypeDefinition<ContentType> {
  return ITEM_CONTENT_TYPE_REGISTRY[contentType] as ItemContentTypeDefinition<ContentType>;
}

export function ItemTypeBadge({ contentType, sizeClassName = "size-9" }: { contentType: ItemContentType; sizeClassName?: string }) {
  const { typeIcon: TypeIcon, accentClassName } = ITEM_CONTENT_TYPE_REGISTRY[contentType];
  return (
    <span className={`flex shrink-0 items-center justify-center rounded-lg ${accentClassName} ${sizeClassName}`}>
      <TypeIcon className="size-[55%]" />
    </span>
  );
}
