# Notif — Real-time Push Notification (Pusher-like)

Sistem push notification real-time berbasis WebSocket dan Redis, dibangun dengan Rust. Mendukung channel publik, private (dengan auth), dan presence (tracking user online).

## Fitur

- **Subscribe / Unsubscribe** ke channel tertentu
- **Broadcast** pesan ke channel via HTTP API
- **Private channel** dengan autentikasi HMAC (Pusher-style)
- **Presence channel** untuk melacak user yang online
- **Redis** sebagai message broker (pub/sub) dan penyimpanan presence
- Arsitektur bersih: handlers, services, repositories, models, middleware, config

## Persyaratan

- Rust 1.70+
- PostgreSQL (untuk dashboard: users, api_keys, domains, channels)
- Redis (pub/sub dan presence)

## Konfigurasi

Salin `.env.example` ke `.env` dan sesuaikan:

```bash
cp .env.example .env
```

Variabel penting:

| Variabel      | Default              | Keterangan                          |
|---------------|----------------------|-------------------------------------|
| `SERVER_ADDR` | `0.0.0.0:3000`       | Bind address server                 |
| `REDIS_URL`   | `redis://127.0.0.1/` | URL koneksi Redis                   |
| `APP_KEY`     | `notif_key`          | Key aplikasi (untuk header API)     |
| `APP_SECRET`  | `notif_secret`       | Secret untuk tanda tangan private/presence |
| `DATABASE_URL`| `postgres://...`      | PostgreSQL untuk dashboard                   |
| `JWT_SECRET`  | (lihat .env.example)  | Secret JWT untuk auth dashboard              |
| `LOG_LEVEL`   | `info`               | Tingkat log (error, warn, info, debug, trace) |

## Menjalankan

```bash
# 1. PostgreSQL: buat DB dan jalankan migrations (lihat docs/SETUP.md)
psql "$DATABASE_URL" -f migrations/001_init_schema.sql

# 2. Redis
redis-server

# 3. Server
cargo run
```

Server mendengarkan di `http://0.0.0.0:3000`. Dashboard: daftar di `/register.html`, login di `/login.html`, lalu akses `/`.

## Build untuk production

```bash
# Build release (optimized, binary di target/release/notif)
cargo build --release
```

Binary hasil build: **`target/release/notif`**.

**Menjalankan di production:**

1. Siapkan environment production (PostgreSQL, Redis, `.env` dengan `DATABASE_URL`, `REDIS_URL`, `APP_KEY`, `APP_SECRET`, `JWT_SECRET`, dll).
2. Jalankan migration sekali: `psql "$DATABASE_URL" -f migrations/001_init_schema.sql`
3. Jalankan binary:
   ```bash
   ./target/release/notif
   ```
   Atau copy binary ke server dan jalankan di sana (mis. dengan systemd):

   ```ini
   # /etc/systemd/system/notif.service
   [Unit]
   Description=Notif WebSocket server
   After=network.target postgresql.service redis.service

   [Service]
   Type=simple
   WorkingDirectory=/opt/notif
   ExecStart=/opt/notif/notif
   Restart=on-failure
   EnvironmentFile=/opt/notif/.env

   [Install]
   WantedBy=multi-user.target
   ```

   ```bash
   sudo systemctl daemon-reload && sudo systemctl enable notif && sudo systemctl start notif
   ```

**Opsional** — memperkecil ukuran binary:

```bash
# Strip symbol (Linux/macOS)
strip target/release/notif
```

## Testing

```bash
cargo test
```

- **Unit tests**: auth (hash/verify/email), channel type, private/presence auth, WebSocket origin/domain matching.
- **Integration tests** (`tests/integration.rs`): health, register+login, broadcast (x-app-key). Untuk integration test yang memakai DB/Redis, set env: `TEST_DATABASE_URL`, `TEST_REDIS_URL` (opsional: `TEST_APP_KEY`, `TEST_APP_SECRET`). Jika env tidak diset, test integration akan di-skip (return tanpa fail).

## API

### WebSocket — `GET /ws`

Sambungkan ke WebSocket; server mengirim `connection_established` dengan `socket_id`.

**Subscribe (kirim JSON):**

```json
{ "event": "subscribe", "data": { "channel": "my-channel" } }
```

**Private channel** (wajib `auth` — HMAC socket_id:channel):

```json
{ "event": "subscribe", "data": { "channel": "private-user-1", "auth": "<hmac_hex>" } }
```

**Presence channel** (wajib `auth` dan optional `channel_data`):

```json
{
  "event": "subscribe",
  "data": {
    "channel": "presence-chat",
    "auth": "<hmac_hex>",
    "channel_data": "{\"user_id\":\"user-123\",\"user_info\":{\"name\":\"Alice\"}}"
  }
}
```

**Unsubscribe:**

```json
{ "event": "unsubscribe", "data": { "channel": "my-channel" } }
```

**Ping:**

```json
{ "event": "ping" }
```

Server merespons dengan `pusher:pong`.

### HTTP — Trigger broadcast

**POST /api/broadcast**

Mengirim event ke sebuah channel. Semua client yang subscribe ke channel tersebut menerima event.

Header:

- `Content-Type: application/json`
- `x-app-key: <APP_KEY>`

Body:

```json
{
  "channel": "my-channel",
  "event": "message",
  "data": { "text": "Hello, world!" }
}
```

Response:

```json
{
  "ok": true,
  "channel": "my-channel",
  "event": "message",
  "subscriber_count": 2
}
```

### Health

**GET /health** — Liveness probe.

## Flow: Subscribe → Publish → Broadcast

1. Client membuka WebSocket ke `ws://localhost:3000/ws`.
2. Client menerima `connection_established` dengan `socket_id`.
3. Client mengirim `subscribe` ke channel (mis. `my-channel`).
4. Server merespons `pusher_internal:subscription_succeeded`.
5. Aplikasi lain (atau script) memanggil `POST /api/broadcast` dengan channel dan event.
6. Server mem-publish ke Redis; semua subscriber channel tersebut menerima event di WebSocket.

## Private / Presence auth

Signature HMAC-SHA256 (hex):

- **Private:** `HMAC(app_secret, socket_id + ":" + channel_name)`
- **Presence:** `HMAC(app_secret, socket_id + ":" + channel_name + ":" + channel_data)`

Contoh menghasilkan auth di backend Anda (mis. Node/Python) atau gunakan contoh client di `examples/` yang memakai secret untuk menghitung signature.

## Dashboard & domain (1 domain = 1 API key)

- **Register** `POST /auth/register` — name, email, password → token
- **Login** `POST /auth/login` — email, password → token
- **Dashboard** (header `Authorization: Bearer <token>`):
  - `GET /dashboard/user` — profil user
  - `GET /dashboard/domains` — list domain (setiap row = 1 domain + 1 API key)
  - `POST /dashboard/domains` — tambah domain (body: `domain_name`) → server generate key
  - `PATCH /dashboard/domains/:id` — aktif/nonaktif (body: `is_active`)
  - `DELETE /dashboard/domains/:id` — hapus domain beserta key-nya
  - `GET /dashboard/channels` — channel milik user
  - `GET /dashboard/ws-status` — koneksi WS aktif per channel

Satu domain = satu API key. WebSocket memakai key tersebut (query `?api_key=...` atau header `x-app-key`); **Origin** request harus cocok dengan **domain_name** domain tersebut.

## Struktur project

```
src/
├── main.rs           # Entry point, wiring
├── lib.rs
├── auth/             # Register, login, JWT
├── config/           # Config dari .env
├── dashboard/        # Handlers dashboard API
├── db/               # Pool + repositories PostgreSQL
├── error/            # AppError
├── handlers/         # HTTP (broadcast, health), WebSocket
├── middleware/       # JWT extractor (AuthUser)
├── models/           # Channel, Event, Presence
├── repositories/     # Redis (pub/sub, presence)
└── services/         # Channel, Presence, Auth (channel HMAC)
migrations/           # SQL schema
dashboard_static/     # Frontend (HTML, Tailwind, jQuery)
docs/                 # API, SETUP
```

## Test

```bash
cargo test
```

## Contoh client

### NotifMoo.js (script tag)

Client JavaScript siap pakai; API key diambil dari query string script:

```html
<script src="notifmoo.js?apikey=radf314df10df1"></script>
```

- File: `dashboard_static/notifmoo.js` (tersedia di `/notifmoo.js` saat server jalan).
- Koneksi WebSocket otomatis ke host yang sama (ws/wss). Opsional: `&host=wss://server.com` jika Notif di server lain dan domain/key mengizinkan.

**API:**

- `NotifMoo.subscribe(channel, { auth?, channelData?, onMessage?(payload) })` — subscribe ke channel (public/private/presence).
- `NotifMoo.unsubscribe(channel)` — unsubscribe.
- `NotifMoo.bind(eventName, callback)` — callback(payload) untuk event tertentu; `*` = semua event.
- `NotifMoo.onConnect(callback)` — dipanggil setelah koneksi dan dapat `socket_id`.
- `NotifMoo.onError(callback)` — callback(err).
- `NotifMoo.socketId`, `NotifMoo.readyState`, `NotifMoo.apikey`.

Contoh lengkap: buka `dashboard_static/notifmoo-example.html` (ganti `YOUR_API_KEY` di src script).

### Lainnya

Lihat `examples/ws_client.html` (browser) atau script di `examples/` untuk menguji WebSocket dan trigger broadcast.
