# LZU EasyTong Read-Only Contract

> Backend contract for LZU EasyTong and campus-service integrations.

---

## Scope

Phase A EasyTong support is read-only:

- Exchange AppService `st` for an EasyTong `EtToken`.
- Query account info.
- Query wallet balance.
- Read and sanitize the AppService service directory.

Payment QR codes, order polling, WebView SSO, cookie injection, and localStorage injection are out of scope until a separate security review task approves them.

---

## Token Handling

- `login_token`, `gateway_token`, `st`, `EtToken`, `AccNum`, `CardAccNum`, and `EPID` stay in Rust runtime memory only.
- Do not persist these values to SQLite, local files, sync snapshots, or frontend state.
- Tauri commands must return low-sensitive projections only. Campus card responses may expose the card tail, never the full card number.
- Logs may include operation names and HTTP status codes, but must not include tokens, full account numbers, request bodies, response bodies, QR payloads, or signatures.

---

## Signing

EasyTong uses MD5 signatures over value order, not map key order. Do not build signatures from `HashMap` iteration.

Required Phase A order:

- `GetAccInfo`: `AccNum -> Time -> md5Key`
- `GetWalletMoney`: `AccNum -> EPID -> Time -> md5Key`

The implementation must keep a unit test for the exact ordered payload shape, including the trailing `|` before the MD5 key.

---

## Response Parsing

- `ExchangeEtToken` returns JSON and must reject missing `token` or `accNum`.
- `GetAccInfo` and `GetWalletMoney` return XML.
- XML parsing must reject an unexpected root element.
- Missing optional XML fields should become `None`; missing or non-numeric `Code` is a parse error.
- Parsing must not panic on missing optional wallet/account fields.

---

## Service Directory

The service directory command returns a sanitized allowlist:

- category id, name, icon URL, sort order
- service id, name, icon URL, category name, introduction, flags, sort order

Do not return H5 URLs, app IDs, sign keys, role/object IDs, process images, contact details, or service-specific auth material in Phase A.

Frontend components must not load remote icon URLs directly. Render local placeholders unless a backend proxy/cache is added in a later task.

---

## Review Checklist

- [ ] EasyTong client is separate from AppService client.
- [ ] No QR code, order polling, payment, WebView SSO, cookie injection, or localStorage injection.
- [ ] No EasyTong/AppService token or full card/account number leaves Rust runtime memory.
- [ ] MD5 signatures use explicit ordered slices and have unit tests.
- [ ] XML parser rejects unexpected roots and tolerates missing optional fields.
- [ ] Service directory sanitizer drops sensitive URL/auth/process fields.
- [ ] Campus-service UI failure is isolated from schedule import and Today.
