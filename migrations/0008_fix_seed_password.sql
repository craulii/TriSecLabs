-- Corrige el hash de contraseña del usuario seed.
-- El hash original en 0007 era incorrecto (no corresponde a "admin123").
-- Este hash fue generado con bcrypt::hash("admin123", 12) desde el crate bcrypt 0.15.
UPDATE users
SET password_hash = '$2b$12$.tLrdHRxPn7MlAsspI2kQOzOc1xdwzy5vykswiWSPxxCcn0R1gjz6'
WHERE email = 'admin@demo.com'
  AND EXISTS (SELECT 1 FROM tenants WHERE slug = 'demo');
