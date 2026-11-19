import en from "./en.json";
import ko from "./ko.json";

export type Locale = "en" | "ko";

const dicts: Record<Locale, any> = { en, ko };

export function detectLocale(): Locale {
  const nav = typeof navigator !== "undefined" ? navigator.language : "";
  return nav.toLowerCase().startsWith("ko") ? "ko" : "en";
}

function get(obj: any, path: string): string | undefined {
  return path.split(".").reduce((acc, key) => (acc == null ? acc : acc[key]), obj);
}

export function createT(locale: Locale) {
  return (key: string, params?: Record<string, string | number>): string => {
    let s = get(dicts[locale], key) ?? get(dicts.en, key) ?? key;
    if (typeof s !== "string") return key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        s = s.replace(new RegExp(`\\{${k}\\}`, "g"), () => String(v));
      }
    }
    return s;
  };
}
