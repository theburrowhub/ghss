# Changelog

## [0.4.0](https://github.com/theburrowhub/ghss/compare/ghss-v0.3.0...ghss-v0.4.0) (2026-07-01)


### Features

* **auth:** endurecer login (gh robusto, scopes, 401 re-login, errores claros) ([0c004eb](https://github.com/theburrowhub/ghss/commit/0c004ebe4c4f427829b4ed458c35bea8fb8a50bf))
* **backend:** tolerar 403/404 de rulesets y filtro por equipos de organización ([17f4e48](https://github.com/theburrowhub/ghss/commit/17f4e48142bc02945abf26d6a6380ad19f39f9cf))
* **branding:** logo y banner en docs + icono de la app ([437c891](https://github.com/theburrowhub/ghss/commit/437c891fb9b2c2300250bc3e323be53c5175f377))
* **dist:** Linux builds (.deb/.AppImage) y sección de instalación con tabs por SO ([3e3f469](https://github.com/theburrowhub/ghss/commit/3e3f469c61ba0ce0f3fefafd3f679af9adee8103))
* **rust:** auth por gh CLI, PAT en keychain y OAuth device flow ([3f19499](https://github.com/theburrowhub/ghss/commit/3f194994144c0ea36e4e342917a5fe6d588a2c12))
* **rust:** cliente GitHub con auth, paginación y reintentos por rate limit ([41dd233](https://github.com/theburrowhub/ghss/commit/41dd2333ba457879e7696e46f094381bd4818e15))
* **rust:** ejecutor de acciones de sync con progreso y errores por acción ([2fd6295](https://github.com/theburrowhub/ghss/commit/2fd6295a340bc551817a2f0a1ae49aaf1633b06c))
* **rust:** fetch de snapshot completo y métodos de escritura ([48e4449](https://github.com/theburrowhub/ghss/commit/48e44494229da69d6f6ec1eb91a3d4792b925e1e))
* **rust:** modelo de settings, diffs y categorías ([192ea80](https://github.com/theburrowhub/ghss/commit/192ea803f92406e53e373ba6363332cd664b248e))
* **rust:** motor de diff de snapshots por categoría y setting ([11766fb](https://github.com/theburrowhub/ghss/commit/11766fb60ccc252058dd34bd6434e0ca39ae4b69))
* **rust:** normalización de rulesets y transform GET→PUT de branch protection ([673e2ad](https://github.com/theburrowhub/ghss/commit/673e2ad66f24caaef6ed3a4431789ffb10f849d2))
* **rust:** planificador de acciones de sync a partir de cambios seleccionados ([8541f6f](https://github.com/theburrowhub/ghss/commit/8541f6f598b8c88c1a9811cfc7678b64c935bebb))
* **sync:** sincronizacion de webhooks de repositorio ([febadfd](https://github.com/theburrowhub/ghss/commit/febadfdbfedf99144f470eb9b4c77321a478fca6))
* **tauri:** comandos de auth, repos, audit y apply_sync con eventos de progreso ([e08b2f4](https://github.com/theburrowhub/ghss/commit/e08b2f40595b23fcf8f39afac3091938d1f98e0f))
* **ui:** auditoría seleccionable, barra de estado al pie y filtro por equipo ([3aaa8f6](https://github.com/theburrowhub/ghss/commit/3aaa8f68c96a3b66d8be041871795a2cc9dd664e))
* **ui:** barra de progreso en la pantalla de sincronización ([33b249f](https://github.com/theburrowhub/ghss/commit/33b249feb0a6af1fe8367c28c9f3e594590eb52e))
* **ui:** botón refrescar listado de repos con purga de caché ([085cb40](https://github.com/theburrowhub/ghss/commit/085cb40dd8f8a7a714adf30e5cbb2220c9f0296e))
* **ui:** diff legible con estado por significado y acción explícita ([d02bfac](https://github.com/theburrowhub/ghss/commit/d02bfacc3caba63ac2f42e4e628e3fc5e032e8cc))
* **ui:** DiffTree con agrupación por categoría y selección tri-estado ([ad62b29](https://github.com/theburrowhub/ghss/commit/ad62b2938d6f3eb358f82b0ea006743f3348e4ea))
* **ui:** fallos por repo no rompen el lote, con mensaje explicativo ([ab17656](https://github.com/theburrowhub/ghss/commit/ab17656d52f871d87edc4b3dae655ee83eb17cec))
* **ui:** mensaje claro de rate limit con tiempo de reset ([9508c03](https://github.com/theburrowhub/ghss/commit/9508c036a07e27867a8dde9e97b7849d434738b9))
* **ui:** no listar repos hasta elegir organización o cuenta ([ffc28d4](https://github.com/theburrowhub/ghss/commit/ffc28d48d8b68702a80e6dc77fabd1c9bf404b9f))
* **ui:** pantalla de carga tras login y auditoría en streaming ([e071085](https://github.com/theburrowhub/ghss/commit/e0710856971404a5d8428ec927049cd1a490fc18))
* **ui:** referencia destacada en panel fijo + selección masiva por filtro ([4bd3412](https://github.com/theburrowhub/ghss/commit/4bd34124c820a90adfa922ad48e49277f7c7e183))
* **ui:** repos archivados ocultos por defecto y no seleccionables como destino ([cc4e800](https://github.com/theburrowhub/ghss/commit/cc4e8006a9d17538bcd5ec18db53871045eb5a12))
* **ui:** tipos, API tipada y tema oscuro estilo Primer ([99ff0a8](https://github.com/theburrowhub/ghss/commit/99ff0a8255271fe3b093cc846dac876421b7547a))
* **ui:** vistas Auth, Repos, Audit, PreSync y Execution con flujo completo ([b4de680](https://github.com/theburrowhub/ghss/commit/b4de6801984eaa72cf18489bcb475bc8d6e6df7d))


### Bug Fixes

* **auth:** aumentar PATH para localizar gh en apps GUI de macOS ([edb90be](https://github.com/theburrowhub/ghss/commit/edb90be3e55e37ebe47a45ebdb84124b970dad60))
* **release:** compilar macOS x86_64 cruzando desde runner arm ([c41a1a8](https://github.com/theburrowhub/ghss/commit/c41a1a8e82a3cebd94b08108d51a520c2c9d39aa))
* **release:** usar TAP_GITHUB_TOKEN (convención org) + concurrency y guard del tap ([82054f0](https://github.com/theburrowhub/ghss/commit/82054f0acb19ed9765c345074938a2782f7437d6))
* **rust:** bound Send en el callback de progreso de apply_actions ([468fddc](https://github.com/theburrowhub/ghss/commit/468fddc692e317deed4d097eac79aa362b47f48d))
* **rust:** contexts, apps y bypass_pull_request_allowances en transform de branch protection ([ff0395b](https://github.com/theburrowhub/ghss/commit/ff0395bd48eff6afe39218e07df4c0f53f9b3046))
* **rust:** guard de payload sin name en plan_actions ([8e65aa1](https://github.com/theburrowhub/ghss/commit/8e65aa18864ec138e289b4f929b4d75e1eb60430))
* **rust:** redacción de token en Debug y validación de device_code ([18d7f01](https://github.com/theburrowhub/ghss/commit/18d7f01f92428d9b8ec22ab767f17ae24bd80417))
* **rust:** robustez del cliente GitHub (errores tipados, drain en retry, headers en tests) ([15b4101](https://github.com/theburrowhub/ghss/commit/15b4101797a193310555da3b0c9453f705d4da5a))
* **rust:** snapshot tolera 404 de protección clásica, pagina rulesets y valida shapes ([b81a242](https://github.com/theburrowhub/ghss/commit/b81a2425d5d38c982451b7f1284889a1f8675eaa))
* **sync:** payload válido de rulesets en la API de escritura (422) ([4062990](https://github.com/theburrowhub/ghss/commit/4062990ebd64d1873e1480f78b425cc1cfde147e))
* **tauri:** orden determinista de diffs y errores en audit ([a66e947](https://github.com/theburrowhub/ghss/commit/a66e94708ea4f7846a2041245e9a50e5e84c039d))
* **tauri:** registra plugin opener y acota concurrencia del audit ([29c3861](https://github.com/theburrowhub/ghss/commit/29c38615c5cf030bf1b14fb75ef53acc8b4f277f))
* tipos de vitest/jest-dom en tsconfig y autoría en Cargo.toml ([e736a3e](https://github.com/theburrowhub/ghss/commit/e736a3e0788c45a968b12a2fa9e7ac26425d6cd6))
* **ui:** detener polling de device flow al cambiar de método de auth ([45d6469](https://github.com/theburrowhub/ghss/commit/45d6469d1c202e7b7b133674dffd2958c4ea38ae))
* **ui:** excluir el repo referencia de los destinos del audit ([199ffb8](https://github.com/theburrowhub/ghss/commit/199ffb8cbb7785babbff4100499409ccb18f09f4))
* **ui:** no descargar repos en el login hasta elegir owner ([ae56fda](https://github.com/theburrowhub/ghss/commit/ae56fdae4ffa5736b72de51e56c1bae47f753497))
* **ui:** persistir filtros entre vistas, deseleccionar todo y hover de fila ([937370f](https://github.com/theburrowhub/ghss/commit/937370fcaef948824fdb62390b4fb143758f77e8))


### Performance Improvements

* **github:** caché de ETags con peticiones condicionales ([9a24e71](https://github.com/theburrowhub/ghss/commit/9a24e714884c38003c803a5124e968f001fd9152))
* **github:** caché de snapshots con TTL e invalidación al escribir ([8b13550](https://github.com/theburrowhub/ghss/commit/8b135506192749d3afb409ab2d49b27a8bff9227))
