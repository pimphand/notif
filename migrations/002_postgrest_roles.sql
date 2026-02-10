-- Optional: roles for PostgREST (run as superuser)
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'notif_anon') THEN
    CREATE ROLE notif_anon NOLOGIN;
  END IF;
END $$;
GRANT USAGE ON SCHEMA public TO notif_anon;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'notif_user') THEN
    CREATE ROLE notif_user NOLOGIN;
  END IF;
END $$;
GRANT USAGE ON SCHEMA public TO notif_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON users TO notif_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON domains TO notif_user;
GRANT SELECT, INSERT ON channels TO notif_user;
GRANT SELECT, INSERT, UPDATE ON ws_connections TO notif_user;
