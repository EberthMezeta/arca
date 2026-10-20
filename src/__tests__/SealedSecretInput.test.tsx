import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";

import "../i18n";
import { changeLanguage } from "../i18n";
import { REDACTED_SECRET_PLACEHOLDER } from "../domain/vaultTypes";
import { vaultApiClient } from "../infrastructure/vaultApiClient";
import { SealedSecretInput, SealedSecretRevealProvider } from "../presentation/editors/SealedSecretInput";

vi.mock("../infrastructure/vaultApiClient", () => ({
  vaultApiClient: {
    revealItemField: vi.fn(async () => "pw-real"),
  },
}));

function EditablePasswordField() {
  const [passwordValue, setPasswordValue] = useState(REDACTED_SECRET_PLACEHOLDER);
  return (
    <SealedSecretRevealProvider itemId="elemento-1">
      <SealedSecretInput aria-label="Contraseña" value={passwordValue} revealableField={{ fieldKind: "password" }} onValueChange={setPasswordValue} />
    </SealedSecretRevealProvider>
  );
}

describe("SealedSecretInput", () => {
  beforeAll(() => changeLanguage("es"));
  beforeEach(() => vi.mocked(vaultApiClient.revealItemField).mockClear());

  it("solo pide el secreto real al pulsar «Mostrar»", async () => {
    const user = userEvent.setup();
    render(<EditablePasswordField />);
    const passwordInput = screen.getByLabelText("Contraseña");
    expect(passwordInput).toHaveValue(REDACTED_SECRET_PLACEHOLDER);
    expect(vaultApiClient.revealItemField).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /mostrar/i }));

    expect(vaultApiClient.revealItemField).toHaveBeenCalledWith("elemento-1", { fieldKind: "password" });
    expect(passwordInput).toHaveValue("pw-real");
    expect(passwordInput).toHaveAttribute("type", "text");
  });

  it("escribir sobre el marcador lo reemplaza en vez de añadirse", async () => {
    const user = userEvent.setup();
    render(<EditablePasswordField />);
    const passwordInput = screen.getByLabelText("Contraseña");

    await user.click(passwordInput);
    await user.keyboard("nueva");

    expect(passwordInput).toHaveValue("nueva");
    expect(vaultApiClient.revealItemField).not.toHaveBeenCalled();
  });
});
