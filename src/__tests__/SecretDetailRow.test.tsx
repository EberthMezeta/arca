import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "@radix-ui/react-tooltip";

import "../i18n";
import { changeLanguage } from "../i18n";
import { REDACTED_SECRET_PLACEHOLDER } from "../domain/vaultTypes";
import { REVEALED_SECRET_VISIBLE_MILLISECONDS, SecretDetailRow } from "../presentation/components/DetailRows";

vi.mock("../infrastructure/vaultApiClient", () => ({
  vaultApiClient: {
    revealItemField: vi.fn(async () => "pw-real"),
  },
}));

describe("SecretDetailRow", () => {
  beforeAll(() => changeLanguage("es"));
  afterEach(() => vi.useRealTimers());

  it("vuelve a ocultar el secreto revelado pasado un tiempo", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(
      <TooltipProvider>
        <SecretDetailRow rowLabel="Contraseña" itemId="elemento-1" revealableField={{ fieldKind: "password" }} maskedValue={REDACTED_SECRET_PLACEHOLDER} />
      </TooltipProvider>,
    );

    await user.click(screen.getByRole("button", { name: /mostrar/i }));
    expect(await screen.findByText("pw-real")).toBeInTheDocument();

    act(() => vi.advanceTimersByTime(REVEALED_SECRET_VISIBLE_MILLISECONDS));

    expect(screen.queryByText("pw-real")).not.toBeInTheDocument();
    expect(screen.getByText(REDACTED_SECRET_PLACEHOLDER)).toBeInTheDocument();
  });
});
