use vault_core::domain::password_strength::PasswordStrength;
use vault_core::domain::secret_text::SecretText;

fn main() {
    for candidate in [
        "MiContraseña2024!",
        "Eberth12345678",
        "contraseñasegura",
        "Perro-Gato-Casa-2024",
        "glaciar-mantis-orbita-terciopelo-27",
        "Hola123456789!",
    ] {
        let strength = PasswordStrength::estimate(&SecretText::new(candidate));
        println!("{candidate:40} score={} log10={:.1}", strength.score, strength.estimated_guesses_log10);
    }
}
