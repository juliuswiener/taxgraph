import os

# Spiegelt server._lade_env_dateien: KEY=VALUE, '#'=Kommentar, Quotes getrimmt, gesetztes Env gewinnt.
for name in (".env.maps", ".env.llm"):
    try:
        with open(name, encoding="utf-8") as f:
            for z in f:
                z = z.strip()
                if not z or z.startswith("#") or "=" not in z:
                    continue
                k, _, v = z.partition("=")
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))
    except OSError:
        pass

print("LLM_API_KEY gesetzt:", bool(os.environ.get("LLM_API_KEY", "").strip()))
print("LLM_API_BASE     :", os.environ.get("LLM_API_BASE", "<FEHLT>"))
print("LLM_MODEL        :", os.environ.get("LLM_MODEL", "<FEHLT>"))
print("ORS_API_KEY gesetzt:", bool(os.environ.get("ORS_API_KEY", "").strip()))
