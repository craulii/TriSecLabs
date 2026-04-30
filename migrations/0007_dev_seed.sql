-- Seed de desarrollo. Crea un tenant demo + usuario admin.
-- Contraseña: "admin123" hasheada con bcrypt cost=12.
-- Para producción: eliminar este archivo o reemplazar con datos reales.
--
-- Regenerar hash:
--   docker run --rm -it alpine sh -c "apk add -q htpasswd && htpasswd -bnBC 12 '' admin123 | tr -d ':\n'"
--   O en Rust: bcrypt::hash("admin123", 12)

DO $$
DECLARE
    v_tenant_id UUID;
    v_user_id   UUID;
BEGIN
    -- Solo insertar si no existe (idempotente)
    IF NOT EXISTS (SELECT 1 FROM tenants WHERE slug = 'demo') THEN

        INSERT INTO tenants (slug, name)
        VALUES ('demo', 'Demo Org')
        RETURNING id INTO v_tenant_id;

        -- bcrypt hash de "admin123" con cost=12
        -- Cambiar en cualquier entorno no-local
        INSERT INTO users (tenant_id, email, password_hash, role)
        VALUES (
            v_tenant_id,
            'admin@demo.com',
            '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5vxou',
            'admin'
        )
        RETURNING id INTO v_user_id;

        -- Target de ejemplo
        PERFORM pg_catalog.set_config('app.tenant_id', v_tenant_id::text, false);
        INSERT INTO scan_targets (tenant_id, kind, name, value)
        VALUES (v_tenant_id, 'domain', 'Demo Corp Website', 'demo.com');

        RAISE NOTICE 'Seed OK — tenant: %, user: admin@demo.com / admin123', v_tenant_id;
    ELSE
        RAISE NOTICE 'Seed already applied, skipping';
    END IF;
END $$;
