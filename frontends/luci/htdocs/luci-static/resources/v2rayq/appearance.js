export const themeChoices = { system: "Follow system", light: "Light", dark: "Dark" };

export function applyTheme(root, value) {
  const theme = Object.hasOwn(themeChoices, value) ? value : "system";
  root.dataset.theme = theme;
  // Embedded plugins style their own surface without changing the host theme.
  if (root.classList.contains("vq-client") && document.body.classList.contains("vq-client-document")) {
    document.documentElement.dataset.theme = theme;
  }
}
