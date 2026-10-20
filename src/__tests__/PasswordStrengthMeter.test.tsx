import { render, screen } from "@testing-library/react";

import "../i18n";
import { changeLanguage } from "../i18n";
import { PasswordStrengthMeter } from "../presentation/components/PasswordStrengthMeter";

describe("PasswordStrengthMeter", () => {
  beforeAll(() => changeLanguage("es"));

  it("muestra la etiqueta traducida según la puntuación", () => {
    render(<PasswordStrengthMeter passwordStrength={{ score: 4, estimatedGuessesLog10: 14, warningMessage: null, improvementSuggestions: [] }} />);

    expect(screen.getByText("Muy fuerte")).toBeInTheDocument();
  });

  it("no muestra nada mientras no hay estimación", () => {
    const { container } = render(<PasswordStrengthMeter passwordStrength={null} />);

    expect(container.textContent).toBe("");
  });
});
