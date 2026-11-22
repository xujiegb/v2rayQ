export async function loadLocalization(base, scope) {
  async function read(url) {
    const response = await fetch(new URL(url, base));
    if (!response.ok) throw new Error("Could not load language resources");
    return response.json();
  }
  const languages = await read("index.json");
  const english = await read("en.json");
  let messages = english;
  let patterns = [];
  let code = "en";
  const cache = new Map([["en", english]]);
  const preferred = () => {
    try { return localStorage.getItem(`v2rayq.${scope}.language`) || navigator.language; }
    catch { return navigator.language; }
  };
  const normalize = value => {
    const requested = (value || navigator.language).replaceAll("_", "-").toLowerCase();
    const exact = languages.find(item => item.code.toLowerCase() === requested);
    if (exact) return exact.code;
    const parts = requested.split("-");
    const candidate = parts[0] === "zh"
      ? (parts.includes("hant") || ["tw", "hk", "mo"].includes(parts.at(-1)) ? "zh-TW" : "zh-CN")
      : parts[0];
    return languages.find(item => item.code.toLowerCase() === candidate.toLowerCase())?.code || "en";
  };
  const api = {
    languages,
    get code() { return code; },
    text(key) { return messages[key] || english[key] || key; },
    format(key, values) {
      return api.text(key).replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name) => values[name] ?? match);
    },
    message(value) {
      if (messages[value] || english[value]) return api.text(value);
      for (const [pattern, names, key] of patterns) {
        const match = pattern.exec(value);
        if (match) return api.format(key, Object.fromEntries(names.map((name, index) => [name, api.text(match[index + 1])])));
      }
      return value;
    },
    bind(element, key, values) {
      element.dataset.i18n = key;
      if (values) element.dataset.i18nValues = JSON.stringify(values);
      element.textContent = values ? api.format(key, values) : api.text(key);
      return element;
    },
    update(root) {
      for (const element of root.querySelectorAll("[data-i18n]")) {
        const values = element.dataset.i18nValues ? JSON.parse(element.dataset.i18nValues) : undefined;
        api.bind(element, element.dataset.i18n, values);
      }
      for (const element of root.querySelectorAll("[data-i18n-placeholder]")) element.placeholder = api.text(element.dataset.i18nPlaceholder);
      for (const element of root.querySelectorAll("[data-i18n-label]")) element.setAttribute("aria-label", api.text(element.dataset.i18nLabel));
    },
    async use(language) {
      const next = normalize(language);
      if (!cache.has(next)) cache.set(next, await read(`${next}.json`));
      code = next;
      messages = cache.get(next);
      patterns = Object.keys(english).filter(key => key.includes("{")).map(key => {
        const names = [];
        const parts = key.split(/(\{[a-zA-Z0-9_]+\})/g);
        const source = parts.map(part => {
          if (/^\{[a-zA-Z0-9_]+\}$/.test(part)) { names.push(part.slice(1, -1)); return "(.+?)"; }
          return part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        }).join("");
        return [new RegExp(`^${source}$`), names, key];
      });
      document.documentElement.lang = code;
      try { localStorage.setItem(`v2rayq.${scope}.language`, code); } catch {}
    }
  };
  await api.use(preferred());
  return api;
}
