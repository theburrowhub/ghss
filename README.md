# ghss — GitHub Settings Sync

Aplicación de escritorio (macOS / Linux) construida con Tauri 2.x que sincroniza la configuración de GitHub desde un repositorio de referencia hacia uno o varios repositorios destino, con auditoría previa y deselección granular de cada cambio.

---

## ¿Qué es ghss?

ghss resuelve un problema habitual en equipos que gestionan múltiples repositorios: mantener la configuración de GitHub consistente entre todos ellos. El flujo es:

1. **Conectar** con tu cuenta de GitHub.
2. **Elegir un repo de referencia** (el que tiene la configuración canónica).
3. **Auditar** el resto de repos para ver cuáles difieren y en qué.
4. **Revisar los diffs** con granularidad por setting antes de aplicar nada.
5. **Sincronizar** solo lo que elijas, repo a repo, setting a setting.

El motor de diff y sync vive íntegramente en Rust (puro, sin Tauri), lo que facilita el testing y garantiza que no se aplique ningún cambio sin haberlo calculado antes.

---

## Flujo de uso

```
Auth → Repos → Auditoría → Pre-sync (diff) → Ejecución
```

| Pantalla | Qué hace |
|---|---|
| **Auth** | Selecciona el método de autenticación y valida el token con GitHub. Muestra login/avatar al autenticar. |
| **Repos** | Lista todos los repos accesibles (personales + orgs). Marca uno como referencia (estrella) y selecciona destinos (checkbox). Repos sin permiso `admin` son visibles pero no seleccionables. |
| **Auditoría** | Compara en paralelo todos los repos listados contra la referencia. Muestra badge `✓ en sync` / `✗ N diferencias` por repo, expandible por categoría. Acción rápida: "seleccionar divergentes como destinos". |
| **Pre-sync** | Árbol Categoría → Setting, solo diferencias, formato `destino → referencia`. Checkbox por setting y por categoría (tri-estado). Confirmación con conteo de cambios y repos afectados. |
| **Ejecución** | Progreso por repo vía eventos Tauri. Errores inline sin abortar el resto. Resumen final: éxitos / warnings / errores. |

---

## Instalación

### macOS

**Homebrew:**

```bash
brew install theburrowhub/tap/ghss
ghss
```

Actualizar: `brew upgrade ghss`. La fórmula instala el binario `ghss`, que abre la interfaz al ejecutarlo.

**O descarga directa:** el `.dmg` de la [última release](https://github.com/theburrowhub/ghss/releases/latest) (Apple Silicon e Intel), ábrelo y arrastra **ghss** a Aplicaciones.

### Linux

Descarga de la [última release](https://github.com/theburrowhub/ghss/releases/latest) el `.deb` (Debian/Ubuntu) o el `.AppImage` portable:

```bash
# Debian / Ubuntu
sudo dpkg -i ghss_*_amd64.deb

# AppImage (cualquier distro)
chmod +x ghss_*.AppImage
./ghss_*.AppImage
```

Requiere el runtime WebKitGTK (`webkit2gtk-4.1`), presente en la mayoría de escritorios modernos.

Documentación completa: **https://theburrowhub.github.io/ghss/**

---

## Requisitos (para compilar)

- **Rust** toolchain stable (instalación recomendada: `rustup`)
- **Node 20+** con npm
- **Sistema operativo**: macOS o Linux (Windows no soportado en v1)
- `gh` CLI instalado y autenticado (opcional, pero es el método de auth por defecto)

---

## Comandos

```bash
# Instalar dependencias de frontend
npm install

# Arrancar en modo desarrollo (Vite HMR + Rust compilado en debug)
npm run tauri dev

# Compilar para producción (genera binario optimizado + app bundle)
npm run tauri build

# Ejecutar suite de tests Rust
cd src-tauri && cargo test

# Ejecutar suite de tests de frontend
npm test
```

---

## Métodos de autenticación

ghss soporta tres métodos, seleccionables desde la pantalla de Auth:

### 1. gh CLI (por defecto)

Si tienes `gh auth login` ejecutado en tu máquina, ghss detecta el token automáticamente invocando `gh auth token`. No requiere ninguna configuración adicional.

### 2. PAT guardado en keychain

Introduce un Personal Access Token de GitHub. ghss lo guarda de forma segura en el keychain del sistema operativo (Keychain en macOS, Secret Service en Linux via `keyring`). No se almacena en disco en texto plano.

El PAT necesita los scopes: `repo`, `read:org`.

### 3. OAuth Device Flow

Requiere registrar una OAuth App propia en GitHub y proporcionar el **Client ID**. ghss inicia el Device Flow, muestra el código de usuario y la URL de autorización, y sondea hasta obtener el token. Este método no requiere que `gh` CLI esté instalado.

Para registrar la OAuth App: GitHub → Settings → Developer settings → OAuth Apps → New OAuth App. No necesita callback URL (es Device Flow).

---

## Categorías sincronizables

| Categoría | Qué incluye |
|---|---|
| **Default branch** | Nombre de la rama por defecto (`default_branch`). Si la rama no existe en el destino, se reporta "no aplicable" sin intentar crearla. |
| **Features** | Wiki, Issues, Projects, Discussions, Allow forking (`has_wiki`, `has_issues`, `has_projects`, `has_discussions`, `allow_forking`) |
| **Pull Requests** | Estrategias de merge (merge commit, squash, rebase), títulos y mensajes de merge commits, allow auto-merge, delete branch on merge, allow update branch |
| **Others** | Requerir firma en commits web (`web_commit_signoff_required`) |
| **Tags** | Rulesets con target `tag` (GitHub migró protected tags a rulesets) |
| **Rules** | Rulesets con target `branch` (conditions, rules, bypass_actors, enforcement) + branch protection clásica para cada rama protegida que exista en el destino |

### Semántica de Rulesets

- Upsert por nombre: si el destino tiene un ruleset con el mismo nombre → `PUT` (actualizar); si no → `POST` (crear).
- No se borran rulesets extra del destino (sync aditivo/correctivo, no destructivo).
- `bypass_actors` con IDs de equipos/apps de la org de referencia que no existan en el destino → se copia el ruleset sin esos actores + warning.

### Semántica de branch protection clásica

- Para cada rama protegida de la referencia que **exista** en el destino: `PUT .../protection` con configuración completa.
- Ramas que no existen en el destino → "no aplicable" + warning.

---

## Limitaciones (sin API pública en v1)

Los siguientes toggles de la UI de GitHub **no se sincronizan** porque GitHub no expone API pública para ellos (o se descartaron en v1). La app los identifica como "no sincronizable (sin API)":

- **Sponsorships** — no existe endpoint de escritura
- **Restrict wiki editing to collaborators** — disponible solo vía GraphQL (descartado en v1)
- **Allow comments on individual commits** — sin API pública
- **Auto-close issues with merged linked PRs** — sin API pública
- **Limit branch/tag updates per push** (push limits) — sin API pública
- **Include Git LFS objects in archives** — sin API pública
- **PR creation allowed by** (permisos de Pull Requests en forks) — sin API pública

---

## Verificación manual pendiente

Las siguientes verificaciones requieren un token válido de GitHub y no pueden ejecutarse en entornos sin acceso a `api.github.com`.

**Prerrequisito**: `gh auth login` con una cuenta real de GitHub.

### Checklist de verificación e2e

- [ ] Abrir la app con `npm run tauri dev`
- [ ] En la pantalla Auth: autenticar con el método gh CLI (o PAT)
- [ ] Verificar que aparece el login/avatar del usuario
- [ ] En la pantalla Repos: confirmar que se listan repos personales y de orgs
- [ ] Marcar un repo de referencia con configuración conocida
- [ ] Ejecutar auditoría contra al menos un repo destino
- [ ] Crear un repo de prueba desechable: `gh repo create ghss-test-target --private`
- [ ] Ejecutar sync contra `ghss-test-target` con algunos settings seleccionados
- [ ] Verificar en GitHub web que los settings se aplicaron correctamente
- [ ] Comprobar que la pantalla de Ejecución muestra éxitos/errores correctamente
- [ ] Limpiar: `gh repo delete ghss-test-target --yes`

> **Nota**: Estos pasos están bloqueados en la verificación automatizada del proyecto porque el token de `gh` CLI disponible en el entorno de CI no tiene acceso válido a `api.github.com`.

---

## Arquitectura interna

```
ghss/
├── src-tauri/           # Backend Rust
│   ├── src/
│   │   ├── github/      # Cliente API GitHub (reqwest): repos, settings, rulesets, branch protection
│   │   ├── auth/        # gh CLI token, PAT + keyring, device flow
│   │   ├── model/       # RepoSettingsSnapshot, SettingsDiff, categorías
│   │   ├── diff/        # Motor de comparación referencia vs destino
│   │   ├── sync/        # Aplicación de cambios (PATCH repos, upsert rulesets, PUT protection)
│   │   └── commands.rs  # Comandos Tauri expuestos al frontend
│   └── tests/           # Unit + integración con wiremock
└── src/                 # Frontend React + TypeScript
    ├── views/           # Auth, Repos, Audit, PreSync, Execution
    ├── components/      # DiffTree, RepoList, CategoryBadge...
    └── api.ts           # invoke() typed wrappers
```

---

## Publicación (release + Homebrew)

El repo lleva dos workflows de GitHub Actions:

- **`pages.yml`** — despliega el sitio `site/` en GitHub Pages en cada push a `main`. Ya activo en https://theburrowhub.github.io/ghss/
- **`release.yml`** — al hacer push de un tag `vX.Y.Z`:
  1. Compila la app con `tauri-action` en macOS (arm64 + x86_64) y publica el `.dmg` en la release.
  2. Empaqueta el binario como `ghss_<version>_<target-triple>.tar.gz` (convención del tap, estilo cargo-dist como `fang`).
  3. Genera/actualiza `Formula/ghss.rb` en `theburrowhub/homebrew-tap`.

### Configuración previa (una sola vez)

El paso 3 necesita un secret en el repo `ghss`:

- **`TAP_GITHUB_TOKEN`**: un PAT con permiso de escritura sobre `theburrowhub/homebrew-tap` (mismo nombre de secret que usan `fang`, `go-secret`… en la org).

```bash
gh secret set TAP_GITHUB_TOKEN --repo theburrowhub/ghss
```

### Cortar una versión

```bash
git tag v0.1.0
git push origin v0.1.0
```

> Nota: el `.dmg` no está firmado/notarizado (requiere credenciales de Apple Developer). En macOS, la primera apertura pide confirmación en Ajustes → Privacidad y seguridad. La instalación vía Homebrew (binario) no tiene esa fricción.

---

## Fuera de alcance (v1)

- Windows
- Sync de webhooks, secrets, labels, colaboradores, environments
- Borrado de rulesets/protecciones sobrantes en repos destino
- Programación/scheduling de syncs automáticos
