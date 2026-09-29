/** Tauri × Gemini header, drawn straight on the background. */
export default function Banner() {
  return (
    <header className="banner" aria-label="Tauri and Gemini, connected by App Functions">
      <img src="/tauri.svg" alt="Tauri" />
      <span className="banner-link" aria-hidden="true">
        <span className="banner-dot" />
        <span className="banner-pill">App Functions</span>
        <span className="banner-dot" />
      </span>
      <img src="/gemini.svg" alt="Gemini" />
    </header>
  );
}
