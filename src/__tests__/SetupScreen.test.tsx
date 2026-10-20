import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import "../i18n";
import { changeLanguage } from "../i18n";
import { vaultApiClient } from "../infrastructure/vaultApiClient";
import { SetupScreen } from "../presentation/screens/SetupScreen";

vi.mock("../infrastructure/vaultApiClient", () => ({
  vaultApiClient: {
    estimatePasswordStrength: vi.fn(async (candidatePassword: string) => ({
      score: candidatePassword.length >= 20 ? 4 : 1,
      estimatedGuessesLog10: 10,
      warningMessage: null,
      improvementSuggestions: [],
    })),
    createVault: vi.fn(async () => undefined),
  },
}));

async function advanceToPasswordStep() {
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: /empezar/i }));
  await user.click(screen.getByLabelText(/entiendo que no se puede recuperar/i));
  await user.click(screen.getByRole("button", { name: /continuar/i }));
  return user;
}

describe("SetupScreen", () => {
  beforeAll(() => changeLanguage("es"));

  it("no deja crear la bóveda hasta que la contraseña es fuerte y coincide", async () => {
    render(<SetupScreen />);
    const user = await advanceToPasswordStep();
    const createButton = screen.getByRole("button", { name: /crear bóveda/i });

    await user.type(screen.getByLabelText("Contraseña maestra"), "corta");
    expect(createButton).toBeDisabled();
    expect(await screen.findByText(/al menos 12 caracteres/i)).toBeInTheDocument();

    await user.clear(screen.getByLabelText("Contraseña maestra"));
    await user.type(screen.getByLabelText("Contraseña maestra"), "glaciar-mantis-orbita-terciopelo");
    await user.type(screen.getByLabelText("Repite la contraseña maestra"), "glaciar-mantis-orbita-terciopelO");
    expect(await screen.findByText("Las contraseñas no coinciden")).toBeInTheDocument();
    expect(createButton).toBeDisabled();

    await user.clear(screen.getByLabelText("Repite la contraseña maestra"));
    await user.type(screen.getByLabelText("Repite la contraseña maestra"), "glaciar-mantis-orbita-terciopelo");
    await vi.waitFor(() => expect(createButton).toBeEnabled());

    await user.click(createButton);
    expect(vaultApiClient.createVault).toHaveBeenCalledWith("glaciar-mantis-orbita-terciopelo");
  });
});
