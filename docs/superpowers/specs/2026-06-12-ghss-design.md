# ghss — GitHub Settings Sync (Design)

**Fecha**: 2026-06-12
**Estado**: Aprobado por el usuario

## Resumen

Aplicación de escritorio (Linux/macOS) que se conecta a GitHub, lista los repositorios accesibles, permite marcar uno como **referencia** y:

1. **Auditar**: ver qué repos de la lista difieren de los settings de la referencia.
2. **Sincronizar**: propagar la configuración de la referencia a un listado de repos elegidos, con deselección granular (por setting individual) antes de aplicar.

## Decisiones aprobadas

| Decisión | Valor |
|---|---|
| Stack | Tauri 2.x — backend Rust, frontend React + TypeScript + Vite |
| Autenticación | 3 métodos a elección del usuario: token de `gh` CLI (default), PAT manual (guardado en keychain), OAuth Device Flow |
| Rules | Ambos sistemas: Rulesets modernos + branch protection clásica |
| Alcance de repos | Cuenta personal + organizaciones (todos los repos con acceso) |
| Granularidad pre-sync | Por setting individual con vista de diff (`valor destino → valor referencia`); solo se muestra/aplica lo que difiere |
| Estética | Tema oscuro inspirado en GitHub Primer |

## Arquitectura

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
└── src/                 # Frontend React + TS
    ├── views/           # Auth, Repos, Audit, PreSync, Execution
    ├── components/      # DiffTree, RepoList, CategoryBadge…
    └── api.ts           # invoke() typed wrappers
```

El motor de diff/sync vive en Rust como módulos puros testeables, independientes de Tauri y de la UI.

### Comandos Tauri

- `auth_status() / auth_login(method, credentials)` — gestión de sesión.
- `list_repos()` — repos personales + de orgs, con flag `admin` por repo.
- `fetch_settings(repo)` — snapshot completo de settings de un repo.
- `audit(reference, targets[])` — snapshots + diffs de todos los targets contra la referencia.
- `apply_sync(reference, plan)` — aplica el plan (lista de cambios seleccionados por repo), emite eventos de progreso.

## Modelo de settings (6 categorías)

| Categoría | Campos / API |
|---|---|
| **Default branch** | `default_branch` vía `PATCH /repos/{o}/{r}`. Si la rama no existe en el destino → se reporta "no aplicable", no se intenta crear. |
| **Features** | `has_wiki`, `has_issues`, `has_projects`, `has_discussions`, `allow_forking` vía `PATCH /repos/{o}/{r}` |
| **Pull Requests** | `allow_merge_commit`, `merge_commit_title`, `merge_commit_message`, `allow_squash_merge`, `squash_merge_commit_title`, `squash_merge_commit_message`, `allow_rebase_merge`, `allow_update_branch`, `allow_auto_merge`, `delete_branch_on_merge` |
| **Others** | `web_commit_signoff_required` |
| **Tags** | Rulesets con `target: tag` (`/repos/{o}/{r}/rulesets`) — GitHub migró protected tags a rulesets |
| **Rules** | Rulesets con `target: branch` (copia íntegra: conditions, rules, bypass_actors, enforcement) **+** branch protection clásica (`GET/PUT /repos/{o}/{r}/branches/{branch}/protection`) de las ramas protegidas de la referencia |

### Limitaciones conocidas (sin API pública)

Estos toggles de la UI de GitHub **no** se sincronizan y la app los lista como "no sincronizable (sin API)":
Sponsorships, "Restrict wiki editing to collaborators" (sí existe vía API GraphQL pero se descarta en v1), "Allow comments on individual commits", "Auto-close issues with merged linked PRs", "Limit branch/tag updates per push", "Include Git LFS objects in archives", "PR creation allowed by" (permissions del toggle Pull requests en forks).

### Semántica de sync de Rulesets

- Upsert por **nombre** de ruleset: si el destino tiene un ruleset con el mismo nombre → `PUT` (actualizar); si no → `POST` (crear).
- No se borran rulesets extra del destino en v1 (sync aditivo/correctivo, no destructivo).
- `bypass_actors` con IDs de equipos/apps de la org de la referencia pueden no existir en el destino → se copia el ruleset sin esos actores y se reporta warning.

### Semántica de branch protection clásica

- Para cada rama protegida de la referencia que **exista** en el destino: `PUT .../protection` con la configuración completa.
- Ramas que no existen en el destino → "no aplicable" + warning.

## Pantallas

1. **Auth**: selector de los 3 métodos; default detecta `gh auth token`. Valida el token y muestra login/avatar. Token PAT se guarda con `keyring` (Keychain en macOS, Secret Service en Linux).
2. **Repos**: lista con buscador y filtro por owner/org. Acciones: marcar referencia (estrella, única), seleccionar destinos (checkbox). Repos sin permiso admin: visibles pero no seleccionables como destino (tooltip explicativo).
3. **Auditoría**: requiere referencia marcada. Botón "Auditar" → fetch paralelo de settings de los repos listados/seleccionados → badge por repo: `✓ en sync` o `✗ N diferencias`, expandible con desglose por categoría. Filtro "solo desincronizados". Acción rápida: "seleccionar divergentes como destinos".
4. **Pre-sync (diff)**: árbol Categoría → Setting, mostrando solo diferencias con formato `destino → referencia`. Checkbox por setting y por categoría (tri-estado). Categorías sin cambios aparecen colapsadas como "(sin cambios)". CTA: "Sincronizar N cambios en M repos".
5. **Ejecución**: progreso por repo (eventos Tauri), resultado por cambio, errores inline sin abortar el resto. Resumen final con éxitos/warnings/errores.

## Manejo de errores

- **Rate limit**: respetar `Retry-After` / `x-ratelimit-reset`, reintentos con backoff; la UI muestra "esperando rate limit".
- **403/404 por permisos o features deshabilitadas**: error por ítem, no aborta el batch.
- **Token expirado**: detectar 401 → volver a pantalla Auth conservando el estado.
- Toda llamada de escritura es idempotente o re-ejecutable sin daño.

## Testing

- **Rust**: unit tests del motor de diff (snapshot vs snapshot) y del mapeo API ⇄ modelo; tests de integración del cliente GitHub con `wiremock` (list, fetch, patch, rulesets upsert, protection).
- **Frontend**: Vitest + Testing Library para DiffTree (selección tri-estado) y flujo de selección de repos.
- **End-to-end**: build del binario, ejecución real contra repos de prueba antes de declarar completo.

## Fuera de alcance (v1)

- Windows.
- Sync de webhooks, secrets, labels, colaboradores, environments.
- Borrado de rulesets/protecciones sobrantes en destinos.
- Programación/scheduling de syncs.
