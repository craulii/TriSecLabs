use common::{AppError, TenantId};
use db::{
    queries::{jobs, ports, scan_targets, vulnerabilities as vulns},
    with_tenant, with_tenant_conn, PgPool,
};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn};
use uuid::Uuid;

struct ScannedPort {
    port:     i32,
    protocol: String,
    service:  Option<String>,
    product:  Option<String>,
    version:  Option<String>,
    vulns:    Vec<ScannedVuln>,
}

struct ScannedVuln {
    cve_id:     Option<String>,
    cvss_score: Option<f64>,
    title:      String,
}

/// Etapas del scan. Se mantienen en sincronía con `common::ScanStage` (snake_case).
struct StageRange {
    name:  &'static str,
    start: i16,
    end:   i16,
}

const STAGES: &[StageRange] = &[
    StageRange { name: "validating",        start: 0,  end: 2  },
    StageRange { name: "starting",          start: 2,  end: 5  },
    StageRange { name: "host_discovery",    start: 5,  end: 10 },
    StageRange { name: "port_scan",         start: 10, end: 40 },
    StageRange { name: "service_detection", start: 40, end: 70 },
    StageRange { name: "vulners",           start: 70, end: 90 },
    StageRange { name: "persisting",        start: 90, end: 99 },
];

fn stage_range(name: &str) -> Option<&'static StageRange> {
    STAGES.iter().find(|s| s.name == name)
}

pub async fn handle(
    pool: &PgPool,
    job_id: Uuid,
    tenant_id: TenantId,
    payload: &Value,
) -> Result<(), AppError> {
    let target_id: Uuid = payload["target_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::InvalidInput("missing target_id in payload".into()))?;

    info!(%tenant_id, %target_id, %job_id, "starting scan");

    // Etapa: validating
    set_stage(pool, job_id, "validating", None).await;

    let target = with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move { scan_targets::find_by_id(conn, target_id).await })
    })
    .await?
    .ok_or_else(|| AppError::not_found(format!("target {target_id}")))?;

    let value = validate_target_value(&target.kind, &target.value)?;

    // Etapa: starting (nmap arrancando)
    set_stage(pool, job_id, "starting", None).await;

    let scanned_ports = run_nmap_streaming(pool, job_id, &value).await?;
    let port_count = scanned_ports.len();
    let tid = tenant_id.0;

    // Etapa: persisting
    set_stage(pool, job_id, "persisting", None).await;

    with_tenant(pool, tenant_id, |tx| {
        Box::pin(async move {
            let mut seen_ids: Vec<Uuid> = Vec::new();

            for p in &scanned_ports {
                let port_id = ports::upsert_port(
                    &mut **tx,
                    tid,
                    target_id,
                    p.port,
                    &p.protocol,
                    "open",
                    p.service.as_deref(),
                    p.product.as_deref(),
                    p.version.as_deref(),
                )
                .await?;
                seen_ids.push(port_id);

                for v in &p.vulns {
                    let fp = make_fingerprint(
                        target_id,
                        p.port,
                        &p.protocol,
                        v.cve_id.as_deref(),
                        &v.title,
                    );
                    let severity = cvss_to_severity(v.cvss_score);
                    let evidence = json!({
                        "port":     p.port,
                        "protocol": p.protocol,
                        "service":  p.service,
                        "product":  p.product,
                        "version":  p.version,
                        "cvss":     v.cvss_score,
                    });
                    vulns::upsert_from_scan(
                        &mut **tx,
                        tid,
                        target_id,
                        Some(port_id),
                        &fp,
                        &v.title,
                        severity,
                        v.cvss_score,
                        v.cve_id.as_deref(),
                        &evidence,
                    )
                    .await?;
                }
            }

            ports::deactivate_stale(&mut **tx, target_id, &seen_ids).await?;

            sqlx::query!(
                "UPDATE scan_targets SET last_scanned_at = now(), updated_at = now() WHERE id = $1",
                target_id
            )
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok(())
        })
    })
    .await?;

    // Si no hay puertos abiertos, anotar en stats para que el usuario lo vea.
    if port_count == 0 {
        let _ = jobs::update_progress(
            pool,
            job_id,
            None,
            None,
            Some(&json!({ "note": "Sin puertos abiertos detectados" })),
        )
        .await;
    }

    jobs::enqueue(
        pool,
        tid,
        "analysis",
        &json!({ "target_id": target_id.to_string() }),
    )
    .await?;

    info!(%target_id, ports = port_count, "scan completed");
    Ok(())
}

// ─── Validación de input ─────────────────────────────────────────────────────

fn domain_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"^(?i)([a-z0-9]([a-z0-9\-]{0,61}[a-z0-9])?\.)+[a-z]{2,}$").unwrap()
    })
}

fn validate_target_value(kind: &str, value: &str) -> Result<String, AppError> {
    let v = value.trim();
    if v.is_empty() {
        return Err(AppError::InvalidInput("Valor vacío".into()));
    }
    match kind {
        "vendor" | "organization" => Err(AppError::InvalidInput(
            "Este tipo de asset no es escaneable directamente. Requiere OSINT manual.".into(),
        )),
        "domain" => {
            if domain_regex().is_match(v) {
                Ok(v.to_lowercase())
            } else {
                Err(AppError::InvalidInput(format!("Dominio inválido: {v}")))
            }
        }
        "ip_range" => {
            if v.contains('/') {
                v.parse::<ipnetwork::IpNetwork>()
                    .map(|_| v.to_string())
                    .map_err(|_| AppError::InvalidInput(format!("CIDR inválido: {v}")))
            } else {
                v.parse::<std::net::IpAddr>()
                    .map(|_| v.to_string())
                    .map_err(|_| AppError::InvalidInput(format!("IP inválida: {v}")))
            }
        }
        _ => Ok(v.to_string()),
    }
}

// ─── Ejecución de nmap con streaming de progreso ─────────────────────────────

async fn set_stage(pool: &PgPool, job_id: Uuid, stage: &str, extra: Option<&Value>) {
    let progress = stage_range(stage).map(|s| s.start);
    if let Err(e) = jobs::update_progress(pool, job_id, progress, Some(stage), extra).await {
        warn!(%job_id, error = %e, "failed to update progress");
    }
}

struct ProgressState {
    stage:            &'static StageRange,
    progress:         i16,
    discovered_ports: Vec<Value>,
    log_tail:         Vec<String>,
}

impl ProgressState {
    fn new() -> Self {
        Self {
            stage:            stage_range("starting").unwrap(),
            progress:         stage_range("starting").unwrap().start,
            discovered_ports: Vec::new(),
            log_tail:         Vec::new(),
        }
    }

    fn enter_stage(&mut self, name: &str) {
        if let Some(s) = stage_range(name) {
            self.stage = s;
            if self.progress < s.start {
                self.progress = s.start;
            }
        }
    }

    fn apply_pct(&mut self, raw_pct: f64) {
        let span = (self.stage.end - self.stage.start) as f64;
        let p = self.stage.start as f64 + (raw_pct.clamp(0.0, 100.0) / 100.0) * span;
        let new = p.round() as i16;
        if new > self.progress {
            self.progress = new.min(self.stage.end);
        }
    }

    fn push_log(&mut self, line: &str) {
        self.log_tail.push(line.to_string());
        if self.log_tail.len() > 50 {
            self.log_tail.remove(0);
        }
    }

    fn snapshot(&self) -> Value {
        json!({
            "discovered_ports": self.discovered_ports,
            "log_tail": self.log_tail,
        })
    }
}

fn re_about_pct() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"About (\d+(?:\.\d+)?)% done").unwrap())
}
fn re_open_port() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"Discovered open port (\d+)/(tcp|udp) on (\S+)").unwrap())
}

fn classify_line(line: &str) -> Option<&'static str> {
    if line.contains("NSE: Loaded") || line.contains("NSE: Script Pre-scanning") {
        // nmap acaba de cargar scripts; sigue en starting pero con señal de vida
        Some("starting")
    } else if line.contains("Initiating Ping Scan")
        || line.contains("Initiating ARP Ping Scan")
        || line.contains("Initiating Parallel DNS resolution")
    {
        Some("host_discovery")
    } else if line.contains("Initiating SYN Stealth Scan")
        || line.contains("Initiating Connect Scan")
        || line.contains("Initiating SYN Scan")
    {
        Some("port_scan")
    } else if line.contains("Initiating Service scan") {
        Some("service_detection")
    } else if line.contains("Script scanning") {
        Some("vulners")
    } else {
        None
    }
}

async fn run_nmap_streaming(
    pool: &PgPool,
    job_id: Uuid,
    value: &str,
) -> Result<Vec<ScannedPort>, AppError> {
    // Estrategia de I/O:
    //  - El XML completo se escribe a un archivo (-oX <path>).
    //  - El progreso textual ("Initiating ...", "Discovered open port ...",
    //    "About X% done") va a stdout cuando -oX no es '-'.
    //  - stderr queda libre para errores reales ("Failed to resolve", etc.).
    //
    // Importante: nmap se ejecuta vía snap en muchos sistemas (Ubuntu/WSL),
    // y el sandbox del snap no puede escribir en /tmp. Usamos $HOME como
    // base — accesible por el confinamiento de snap.
    let xml_path = nmap_temp_path(job_id);
    if let Some(parent) = xml_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let xml_path_str = xml_path.to_string_lossy().to_string();

    let mut child = Command::new("nmap")
        .args([
            "-sV",
            "--script=vulners",
            "--script-args=mincvss=5.0",
            "-T4",
            "--open",
            "-v",
            "--stats-every",
            "3s",
            "-oX",
            &xml_path_str,
            value,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::internal("nmap no está instalado en el sistema")
            } else {
                AppError::internal(format!("nmap execution failed: {e}"))
            }
        })?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let mut state = ProgressState::new();
    let mut last_persist = Instant::now() - Duration::from_secs(2);
    let mut stderr_buffer = String::new();
    let mut stdout_buffer = String::with_capacity(8 * 1024);

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let timeout_at = Instant::now() + Duration::from_secs(600);

    let mut stdout_done = false;
    let mut stderr_done = false;
    // Keepalive: cada 5s emitimos un update aunque nmap esté en silencio.
    // Da sensación de progreso al usuario durante NSE pre-scanning, DNS resolution,
    // etc. — fases que nmap procesa sin escribir a stderr.
    let mut keepalive = tokio::time::interval(Duration::from_secs(5));
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    keepalive.tick().await; // descarta el tick inmediato

    loop {
        if Instant::now() > timeout_at {
            let _ = child.start_kill();
            return Err(AppError::internal(
                "Scan excedió 10 minutos — host demasiado lento o filtrado",
            ));
        }

        tokio::select! {
            line = stdout_reader.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => {
                    // stdout contiene las líneas de progreso de nmap cuando -oX
                    // apunta a un archivo. Las usamos para detectar etapa,
                    // porcentaje y puertos descubiertos en tiempo real.
                    if stdout_buffer.len() < 8192 {
                        stdout_buffer.push_str(&l);
                        stdout_buffer.push('\n');
                    }
                    process_line(pool, job_id, &mut state, &mut last_persist, &l).await;
                }
                Ok(None) => stdout_done = true,
                Err(e)   => {
                    warn!(error = %e, "stdout read error");
                    stdout_done = true;
                }
            },
            line = stderr_reader.next_line(), if !stderr_done => match line {
                Ok(Some(l)) => {
                    if stderr_buffer.len() < 8192 {
                        stderr_buffer.push_str(&l);
                        stderr_buffer.push('\n');
                    }
                    // Algunas versiones aún emiten progreso por stderr; lo procesamos también.
                    process_line(pool, job_id, &mut state, &mut last_persist, &l).await;
                }
                Ok(None) => stderr_done = true,
                Err(e)   => {
                    warn!(error = %e, "stderr read error");
                    stderr_done = true;
                }
            },
            _ = keepalive.tick() => {
                // Avanza el progress lentamente dentro del rango de la fase actual
                // para mostrar que el scan sigue vivo aunque nmap esté en silencio.
                let max = state.stage.end.saturating_sub(2);
                if state.progress < max {
                    state.progress = (state.progress + 1).min(max);
                    let _ = jobs::update_progress(
                        pool,
                        job_id,
                        Some(state.progress),
                        Some(state.stage.name),
                        Some(&state.snapshot()),
                    ).await;
                }
            },
            res = child.wait(), if stdout_done && stderr_done => {
                let status = res.map_err(|e| AppError::internal(format!("nmap wait failed: {e}")))?;
                if !status.success() {
                    return Err(map_nmap_error(status.code(), &stderr_buffer));
                }
                break;
            }
        }
    }

    // nmap retorna exit 0 incluso cuando falla la resolución DNS o el host
    // está down. Detectamos esos casos buscando en stdout y stderr (los
    // mensajes informativos pueden ir a cualquiera dependiendo de la versión).
    let combined = format!("{stderr_buffer}\n{stdout_buffer}");
    if let Some(err) = detect_silent_failure(&combined) {
        let _ = std::fs::remove_file(&xml_path);
        return Err(err);
    }

    // Snapshot final antes del parseo (etapa persisting la setea el caller)
    let _ = jobs::update_progress(
        pool,
        job_id,
        Some(state.stage.end.min(89)),
        Some(state.stage.name),
        Some(&state.snapshot()),
    )
    .await;

    let xml = match tokio::fs::read_to_string(&xml_path).await {
        Ok(s) => s,
        Err(e) => {
            return Err(AppError::internal(format!(
                "no se pudo leer XML de nmap ({}): {e}",
                xml_path.display()
            )));
        }
    };
    let _ = std::fs::remove_file(&xml_path);

    if xml.trim().is_empty() {
        return Ok(vec![]);
    }
    parse_nmap_xml(&xml)
}

/// Path único por job para el XML de salida. Usa `$HOME/triseclabs-tmp/` en
/// lugar de `/tmp` porque nmap (snap) no puede escribir fuera de su sandbox.
fn nmap_temp_path(job_id: Uuid) -> std::path::PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(base)
        .join("triseclabs-tmp")
        .join(format!("scan-{job_id}.xml"))
}

/// Detecta condiciones de fallo de nmap que NO se reflejan en exit code:
/// "Failed to resolve" (DNS), "Host seems down" (host filtrado), "0 hosts up".
fn detect_silent_failure(stderr: &str) -> Option<AppError> {
    if stderr.contains("Failed to resolve") {
        return Some(AppError::InvalidInput(
            "DNS no resuelve el destino. Verifica el dominio.".into(),
        ));
    }
    if stderr.contains("Host seems down") {
        return Some(AppError::InvalidInput(
            "El host no responde a ping. Probablemente está detrás de un firewall que bloquea ICMP. \
             Verifica que el host sea accesible públicamente."
                .into(),
        ));
    }
    if stderr.contains("0 hosts up") || stderr.contains("0 IP addresses") {
        return Some(AppError::InvalidInput(
            "Ningún host respondió. Verifica el destino y la conectividad.".into(),
        ));
    }
    None
}

async fn process_line(
    pool: &PgPool,
    job_id: Uuid,
    state: &mut ProgressState,
    last_persist: &mut Instant,
    line: &str,
) {
    let line_trim = line.trim();
    if line_trim.is_empty() {
        return;
    }

    let mut changed = false;

    if let Some(stage_name) = classify_line(line_trim) {
        state.enter_stage(stage_name);
        changed = true;
    }

    if let Some(c) = re_about_pct().captures(line_trim) {
        if let Ok(pct) = c.get(1).unwrap().as_str().parse::<f64>() {
            state.apply_pct(pct);
            changed = true;
        }
    }

    if let Some(c) = re_open_port().captures(line_trim) {
        let port = c.get(1).unwrap().as_str().parse::<i32>().unwrap_or(0);
        let protocol = c.get(2).unwrap().as_str().to_string();
        if port > 0 {
            state.discovered_ports.push(json!({
                "port": port,
                "protocol": protocol,
            }));
            state.push_log(line_trim);
            changed = true;
        }
    }

    // Throttle: persistir como mucho 1 vez por segundo
    if changed && last_persist.elapsed() >= Duration::from_secs(1) {
        let _ = jobs::update_progress(
            pool,
            job_id,
            Some(state.progress),
            Some(state.stage.name),
            Some(&state.snapshot()),
        )
        .await;
        *last_persist = Instant::now();
    }
}

fn map_nmap_error(code: Option<i32>, stderr: &str) -> AppError {
    if stderr.contains("Failed to resolve") {
        AppError::InvalidInput("DNS no resuelve el destino. Verifica el dominio.".into())
    } else if stderr.contains("Note: Host seems down") || stderr.contains("Host seems down") {
        AppError::InvalidInput(
            "El host no responde. Verifica conectividad o usa una IP/dominio accesible.".into(),
        )
    } else if stderr.contains("dnet: Failed to open device")
        || stderr.contains("Operation not permitted")
    {
        AppError::Internal("nmap requiere permisos elevados para este tipo de scan".into())
    } else {
        let tail: String = stderr.lines().rev().take(5).collect::<Vec<_>>().join(" | ");
        AppError::Internal(format!(
            "nmap exit {}: {}",
            code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            tail.trim()
        ))
    }
}

fn parse_nmap_xml(xml: &str) -> Result<Vec<ScannedPort>, AppError> {
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(xml, opts)
        .map_err(|e| AppError::internal(format!("nmap XML parse error: {e}")))?;

    let mut result = Vec::new();

    for host in doc.descendants().filter(|n| n.has_tag_name("host")) {
        for port_node in host.descendants().filter(|n| n.has_tag_name("port")) {
            let state = port_node
                .children()
                .find(|n| n.has_tag_name("state"))
                .and_then(|n| n.attribute("state"))
                .unwrap_or("");
            if state != "open" {
                continue;
            }

            let protocol = port_node.attribute("protocol").unwrap_or("tcp").to_string();
            let port_num: i32 = match port_node
                .attribute("portid")
                .and_then(|s| s.parse().ok())
            {
                Some(p) if p > 0 => p,
                _ => continue,
            };

            let svc = port_node.children().find(|n| n.has_tag_name("service"));
            let service = svc.and_then(|n| n.attribute("name")).map(str::to_string);
            let product = svc.and_then(|n| n.attribute("product")).filter(|s| !s.is_empty()).map(str::to_string);
            let version = svc.and_then(|n| n.attribute("version")).filter(|s| !s.is_empty()).map(str::to_string);

            let mut port_vulns = Vec::new();

            if let Some(script) = port_node
                .children()
                .find(|n| n.has_tag_name("script") && n.attribute("id") == Some("vulners"))
            {
                for outer in script.children().filter(|n| n.has_tag_name("table")) {
                    for cve_tbl in outer.children().filter(|n| n.has_tag_name("table")) {
                        let key = cve_tbl.attribute("key").unwrap_or("");
                        let mut cvss: Option<f64> = None;
                        let mut cve_id: Option<String> = None;

                        for elem in cve_tbl.children().filter(|n| n.has_tag_name("elem")) {
                            match elem.attribute("key") {
                                Some("cvss") => cvss = elem.text().and_then(|s| s.parse().ok()),
                                Some("id")   => cve_id = elem.text().map(str::to_string),
                                _            => {}
                            }
                        }

                        let title = cve_id.clone().unwrap_or_else(|| key.to_string());
                        if !title.is_empty() {
                            port_vulns.push(ScannedVuln { cve_id, cvss_score: cvss, title });
                        }
                    }
                }
            }

            result.push(ScannedPort { port: port_num, protocol, service, product, version, vulns: port_vulns });
        }
    }

    Ok(result)
}

fn make_fingerprint(target_id: Uuid, port: i32, protocol: &str, cve_id: Option<&str>, title: &str) -> String {
    let mut h = Sha256::new();
    h.update(target_id.as_bytes());
    h.update(format!(":{protocol}:{port}:").as_bytes());
    h.update(cve_id.unwrap_or(title).as_bytes());
    format!("{:x}", h.finalize())
}

fn cvss_to_severity(cvss: Option<f64>) -> &'static str {
    match cvss {
        Some(s) if s >= 9.0 => "critical",
        Some(s) if s >= 7.0 => "high",
        Some(s) if s >= 4.0 => "medium",
        Some(s) if s >  0.0 => "low",
        _                   => "info",
    }
}
