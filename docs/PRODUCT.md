# TriSecLabs — Documentación de Producto

> Plataforma de gestión continua de exposición a amenazas (CTEM) para empresas que necesitan visibilidad real de su superficie de ataque externa.

---

## ¿Qué es TriSecLabs?

TriSecLabs es una plataforma de ciberseguridad que permite a las organizaciones **monitorear, detectar y gestionar continuamente las vulnerabilidades** de sus activos digitales externos: sitios web, servidores, rangos de IP, proveedores tecnológicos y entidades corporativas.

El sistema combina tres capacidades en una sola plataforma:

1. **Escaneo de activos** — detecta puertos expuestos, servicios activos y vulnerabilidades conocidas
2. **Scoring de riesgo automatizado** — calcula un índice de riesgo (0–100) por activo basado en la gravedad de los hallazgos
3. **Informes ejecutivos con IA** — genera reportes en español, redactados por un modelo de lenguaje local (sin enviar datos a la nube)

---

## ¿Por qué TriSecLabs?

Las organizaciones hoy enfrentan un problema común: **saben que tienen vulnerabilidades, pero no saben cuáles son las más críticas ni cómo priorizarlas.**

TriSecLabs resuelve esto con una interfaz clara que muestra exactamente qué activos están en riesgo, qué vulnerabilidades tienen, y un informe generado automáticamente que cualquier directivo puede entender.

**Además, toda la información permanece en sus servidores.** El modelo de IA corre localmente — ningún dato de vulnerabilidades sale de la organización.

---

## Casos de uso

### Para equipos de seguridad (SOC / Blue Team)
- Monitorear todos los activos externos desde un único panel
- Priorizar remediaciones por score de riesgo
- Gestionar el ciclo de vida de cada vulnerabilidad (open → in_progress → resolved)
- Detectar nuevos puertos expuestos tras cambios en infraestructura

### Para CISOs y directivos
- Obtener informes ejecutivos en lenguaje claro, sin tecnicismos
- Ver tendencias históricas de vulnerabilidades (¿está mejorando la postura de seguridad?)
- Demostrar postura de seguridad a auditores y clientes

### Para equipos de cumplimiento
- Evidencia documental de vulnerabilidades detectadas y remediadas
- Historial de cambios de estado con notas de remediación
- Scoring continuo para reportes regulatorios (ISO 27001, ENS, etc.)

### Para MSSPs (proveedores de servicios de seguridad)
- Arquitectura multi-tenant: cada cliente ve únicamente sus propios datos
- Panel unificado para gestionar múltiples organizaciones
- Roles diferenciados: `admin` (gestión completa) y `analyst` (operación)

---

## Funcionalidades principales

### Panel de control (Dashboard)

Visión general instantánea de la postura de seguridad:

- **KPIs en tiempo real:** total de activos, riesgo promedio, vulnerabilidades críticas y altas
- **Gráfico de tendencias:** evolución de vulnerabilidades por severidad en los últimos 30 días
- **Creación rápida de activos:** añade dominios, rangos IP, proveedores u organizaciones directamente desde el dashboard

### Gestión de activos

Lista completa de todos los activos monitoreados con:

- Nombre, tipo y valor técnico del activo (dominio, IP, proveedor, organización)
- Nivel de riesgo actual (crítico / alto / medio / bajo / info)
- Fecha del último escaneo
- Acciones directas: lanzar escaneo o generar informe LLM

### Detalle de activo

Vista completa de un activo individual:

- **Gauge de riesgo** — indicador visual 0–100
- **Métricas KPI** — número de vulnerabilidades críticas, altas y puertos expuestos
- **Evolución histórica** — gráfico de tendencia de 30 días
- **Puertos expuestos** — tabla completa con servicio, producto y versión detectados
- **Vulnerabilidades** — lista ordenada por severidad con CVE, estado y fuente

### Gestión de vulnerabilidades

Panel global de todas las vulnerabilidades de la organización:

- **Filtros** por severidad, estado y búsqueda de texto libre
- **Información completa:** título, CVE/CWE, score CVSS, fecha de detección
- **Cambio de estado** directo desde la tabla: open → in_progress → mitigated → resolved → accepted → false_positive
- **Paginación** para organizaciones con grandes volúmenes de hallazgos

### Informes LLM (Inteligencia Artificial)

Generación automática de informes ejecutivos en español:

- El informe describe los hallazgos más críticos del activo en lenguaje natural
- Redactado por un modelo de lenguaje (Mistral-7B-Instruct) que corre localmente
- **Privacidad total:** ningún dato sale de los servidores de la organización
- El informe puede ser entregado directamente a dirección sin modificaciones técnicas

---

## Niveles de riesgo

| Nivel | Color | Criterio |
|---|---|---|
| **Crítico** | Rojo | Score ≥ 75 / Vulnerabilidades CVSS 9.0+ |
| **Alto** | Naranja | Score 50–74 / Vulnerabilidades CVSS 7.0–8.9 |
| **Medio** | Amarillo | Score 25–49 / Vulnerabilidades CVSS 4.0–6.9 |
| **Bajo** | Verde | Score 10–24 / Vulnerabilidades CVSS 0.1–3.9 |
| **Info** | Gris | Score < 10 / Hallazgos informativos |

El score de riesgo (0–100) se calcula automáticamente ponderando las vulnerabilidades detectadas:
- Crítica × 25 puntos
- Alta × 10 puntos
- Media × 4 puntos
- Baja × 1 punto

---

## Tipos de activos soportados

| Tipo | Descripción | Ejemplo |
|---|---|---|
| **Dominio** | Sitio web o dominio DNS | `example.com`, `api.empresa.com` |
| **Rango IP** | CIDR o IP individual | `192.168.1.0/24`, `203.0.113.5` |
| **Proveedor** | Tercero tecnológico | Microsoft Azure, AWS, Salesforce |
| **Organización** | Entidad corporativa | Filiales, subsidiarias, holding |

---

## Fuentes de vulnerabilidades

TriSecLabs puede integrar hallazgos de múltiples fuentes:

| Fuente | Tipo |
|---|---|
| `nmap` | Escaneo activo de puertos y servicios |
| `nessus` | Scanner comercial de vulnerabilidades |
| `openvas` | Scanner open-source de vulnerabilidades |
| `manual` | Introducción manual por el analista |
| `llm_analysis` | Detectado por análisis de IA |
| `osint` | Fuentes de inteligencia pública |

---

## Roles de usuario

| Rol | Capacidades |
|---|---|
| **Admin** | Acceso completo: crear activos, lanzar scans, gestionar vulnerabilidades, ver informes, administrar usuarios y tenants |
| **Analyst** | Operación: lanzar scans, generar informes, actualizar estados de vulnerabilidades; sin gestión de usuarios |

---

## Arquitectura de privacidad

TriSecLabs está diseñado para máxima privacidad y control de datos:

- **Despliegue on-premise:** toda la plataforma corre en la infraestructura del cliente
- **LLM local:** el modelo de IA no requiere conexión a internet ni envía datos a terceros
- **Aislamiento multi-tenant:** cada organización tiene sus datos completamente separados a nivel de base de datos (Row Level Security en PostgreSQL)
- **Sin telemetría:** la plataforma no envía métricas ni estadísticas a ningún servidor externo

---

## Modalidades de uso

### Aplicación web
Accesible desde cualquier navegador moderno. Ideal para equipos remotos o acceso desde múltiples dispositivos.

### Aplicación de escritorio (Tauri)
Aplicación nativa para Windows, macOS y Linux. El servidor API corre integrado — no requiere conexión a internet para operar. Ideal para entornos air-gapped o con políticas de seguridad restrictivas.

---

## Requisitos de despliegue

### Mínimos recomendados
- CPU: 4 núcleos
- RAM: 8 GB (16 GB si se usa LLM)
- Almacenamiento: 50 GB SSD
- OS: Linux (Ubuntu 22.04+ recomendado) / Windows / macOS

### Para el módulo LLM (opcional)
- RAM adicional: 8 GB mínimo para Mistral-7B en CPU
- GPU NVIDIA (opcional): acelera significativamente la generación de informes
- El servidor LLM puede separarse del servidor principal en infraestructuras grandes

### Software requerido
- Docker + Docker Compose (para despliegue rápido)
- PostgreSQL 16 (incluido en Docker Compose)

---

## Hoja de ruta

### Entregado

- ✅ **Integración nmap activa** — escaneo con `-sV` + script `vulners` para detección de CVEs por servicio
- ✅ **Scan con progreso en vivo** — drawer con timeline de etapas, puertos descubiertos en tiempo real y log de nmap (vía Server-Sent Events)
- ✅ **Validación robusta de targets** — feedback claro ante DNS fail, host inalcanzable, timeout, o tipo de asset no escaneable
- ✅ **Retry inteligente** — errores transitorios se reencolan automáticamente; errores de input fallan inmediato

### Próximas funcionalidades

- **Scans programados** — recurrencia configurable por activo (diario, semanal, mensual)
- **Integración Shodan** — visibilidad de activos indexados públicamente
- **Gestión completa de usuarios** — invitaciones, cambio de contraseña, 2FA
- **Gestión de tenants** — creación y configuración desde la UI
- **Historial de informes LLM** — archivo de informes anteriores por activo
- **Notificaciones** — alertas por email o webhook cuando se detectan nuevas vulnerabilidades críticas
- **Exportación PDF** — informes ejecutivos listos para presentar
- **API pública** — integración con SIEM y otras herramientas de seguridad

---

## Glosario

| Término | Definición |
|---|---|
| **CTEM** | Continuous Threat Exposure Management — gestión continua de exposición a amenazas |
| **Asset / Activo** | Cualquier recurso digital externo de la organización bajo monitoreo |
| **Scan** | Escaneo técnico de un activo para detectar puertos y vulnerabilidades |
| **CVE** | Common Vulnerabilities and Exposures — identificador único de vulnerabilidades conocidas |
| **CVSS** | Common Vulnerability Scoring System — estándar de puntuación 0–10 de severidad |
| **CWE** | Common Weakness Enumeration — clasificación de debilidades de software |
| **Risk Score** | Puntuación de riesgo agregada (0–100) calculada automáticamente por TriSecLabs |
| **Tenant** | Organización cliente con su propio espacio de datos aislado |
| **LLM** | Large Language Model — modelo de lenguaje de IA para generación de informes |
| **RLS** | Row Level Security — mecanismo PostgreSQL de aislamiento de datos por tenant |

---

*TriSecLabs v0.1.0 — Plataforma CTEM con IA local*
