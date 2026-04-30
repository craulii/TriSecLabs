<script lang="ts">
  import type { RiskLevel, VulnStatus } from '$lib/types/models';

  let {
    value,
    variant = 'severity',
  }: { value: RiskLevel | VulnStatus; variant?: 'severity' | 'status' } = $props();

  const SEVERITY_MAP: Record<RiskLevel, { label: string; color: string }> = {
    critical: { label: 'Critical', color: 'var(--sev-critical)' },
    high:     { label: 'High',     color: 'var(--sev-high)' },
    medium:   { label: 'Medium',   color: 'var(--sev-medium)' },
    low:      { label: 'Low',      color: 'var(--sev-low)' },
    info:     { label: 'Info',     color: 'var(--sev-info)' },
  };

  const STATUS_MAP: Record<VulnStatus, { label: string; color: string }> = {
    open:           { label: 'Open',           color: 'var(--sev-critical)' },
    in_progress:    { label: 'In Progress',    color: 'var(--sev-medium)' },
    mitigated:      { label: 'Mitigated',      color: '#8b5cf6' },
    resolved:       { label: 'Resolved',       color: 'var(--sev-low)' },
    accepted:       { label: 'Accepted',       color: 'var(--text-muted)' },
    false_positive: { label: 'False Positive', color: 'var(--text-muted)' },
  };

  const map = variant === 'severity' ? SEVERITY_MAP : STATUS_MAP;
  const entry = $derived(
    (map as Record<string, { label: string; color: string }>)[value] ?? { label: value, color: 'var(--text-muted)' }
  );
</script>

<span
  class="inline-flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full"
  style="
    background: color-mix(in srgb, {entry.color} 15%, transparent);
    color: {entry.color};
    border: 1px solid color-mix(in srgb, {entry.color} 30%, transparent);
  "
>
  <span
    class="w-1.5 h-1.5 rounded-full flex-shrink-0"
    style="background: {entry.color};"
  ></span>
  {entry.label}
</span>
