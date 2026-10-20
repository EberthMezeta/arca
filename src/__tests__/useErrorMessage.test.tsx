import { renderHook } from "@testing-library/react";

import "../i18n";
import { useErrorMessage } from "../application/hooks/useErrorMessage";
import { changeLanguage } from "../i18n";

describe("useErrorMessage", () => {
  it("traduce los códigos de detalle de entrada inválida", () => {
    changeLanguage("en");
    const { result: describeError } = renderHook(() => useErrorMessage());

    expect(describeError.current({ errorCode: "invalidInput", technicalDetail: "folderNameDuplicated" })).toBe(
      "A folder with that name already exists.",
    );
  });

  it("interpola los segundos de espera tras demasiados intentos", () => {
    changeLanguage("es");
    const { result: describeError } = renderHook(() => useErrorMessage());

    expect(describeError.current({ errorCode: "unlockTemporarilyBlocked", retryAfterSeconds: 8 })).toBe("Demasiados intentos. Espera 8 s.");
  });

  it("usa un mensaje genérico para códigos desconocidos", () => {
    changeLanguage("es");
    const { result: describeError } = renderHook(() => useErrorMessage());

    expect(describeError.current({ errorCode: "somethingNew" })).toBe("Algo salió mal. Inténtalo de nuevo.");
  });
});
