-- Campos para progreso en vivo de jobs (scan principalmente).
-- Los jobs preexistentes tienen progress=NULL; el cliente lo trata como "sin datos".

ALTER TABLE jobs
    ADD COLUMN progress     SMALLINT,
    ADD COLUMN current_step TEXT,
    ADD COLUMN stats_json   JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Índice parcial para SSE: el stream consulta updated_at frecuentemente
CREATE INDEX idx_jobs_running_recent
    ON jobs (updated_at DESC)
    WHERE status = 'running';
