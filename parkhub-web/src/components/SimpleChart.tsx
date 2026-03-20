interface BarChartProps {
  data: { label: string; value: number }[];
  height?: number;
  color?: string;
}

// ── Donut Chart ──────────────────────────────────────────────────────────────

export interface DonutSlice {
  label: string;
  /** 0–100 occupancy percent for this lot */
  occupancy: number;
  /** Number of total spaces (used for weighting the arc) */
  capacity: number;
}

interface DonutChartProps {
  slices: DonutSlice[];
  size?: number;
  strokeWidth?: number;
}

function occupancyColor(pct: number): string {
  if (pct >= 80) return 'var(--color-red-500, #ef4444)';
  if (pct >= 60) return 'var(--color-amber-400, #fbbf24)';
  return 'var(--color-emerald-500, #10b981)';
}

/**
 * Pure-SVG donut chart. Each slice is sized by lot capacity; color reflects
 * occupancy level: green <60 %, yellow 60–80 %, red ≥80 %.
 * Center text shows weighted-average total occupancy %.
 */
export function DonutChart({ slices, size = 200, strokeWidth = 28 }: DonutChartProps) {
  if (slices.length === 0) return null;

  const r = (size - strokeWidth) / 2;
  const cx = size / 2;
  const cy = size / 2;
  const circumference = 2 * Math.PI * r;

  const totalCapacity = slices.reduce((s, d) => s + Math.max(d.capacity, 1), 0);
  const totalOccupied = slices.reduce((s, d) => s + d.occupancy * Math.max(d.capacity, 1) / 100, 0);
  const overallPct = totalCapacity > 0 ? Math.round(totalOccupied / totalCapacity * 100) : 0;

  // Build arc segments — each slice arc length proportional to its capacity
  const GAP = 2; // px gap between arcs
  const arcs: { offset: number; dash: number; color: string; label: string; occupancy: number }[] = [];
  let consumed = 0;

  for (const slice of slices) {
    const weight = Math.max(slice.capacity, 1) / totalCapacity;
    const arcLen = weight * circumference;
    const gapFrac = Math.min(GAP, arcLen * 0.5);
    arcs.push({
      offset: circumference - consumed + gapFrac / 2,
      dash: arcLen - gapFrac,
      color: occupancyColor(slice.occupancy),
      label: slice.label,
      occupancy: slice.occupancy,
    });
    consumed += arcLen;
  }

  const overallColor = occupancyColor(overallPct);

  return (
    <svg
      role="img"
      aria-label={`Donut chart: overall occupancy ${overallPct}%`}
      width={size}
      height={size}
      className="block"
    >
      {/* Background track */}
      <circle
        cx={cx} cy={cy} r={r}
        fill="none"
        stroke="var(--theme-bg-muted)"
        strokeWidth={strokeWidth}
      />

      {arcs.map((arc, i) => (
        <circle
          key={i}
          cx={cx} cy={cy} r={r}
          fill="none"
          stroke={arc.color}
          strokeWidth={strokeWidth}
          strokeDasharray={`${arc.dash} ${circumference - arc.dash}`}
          strokeDashoffset={arc.offset}
          strokeLinecap="round"
        >
          <title>{arc.label}: {arc.occupancy}%</title>
        </circle>
      ))}

      {/* Center label */}
      <text
        x={cx} y={cy - 8}
        textAnchor="middle"
        dominantBaseline="middle"
        fontSize={28}
        fontWeight={700}
        fill={overallColor}
      >
        {overallPct}%
      </text>
      <text
        x={cx} y={cy + 18}
        textAnchor="middle"
        dominantBaseline="middle"
        fontSize={11}
        fontWeight={500}
        className="fill-surface-500"
      >
        occupancy
      </text>
    </svg>
  );
}

const PADDING_LEFT = 80;
const PADDING_RIGHT = 48;
const PADDING_TOP = 8;
const PADDING_BOTTOM = 4;
const BAR_HEIGHT = 28;
const BAR_GAP = 10;

export function BarChart({ data, height, color = 'var(--color-primary-500)' }: BarChartProps) {
  if (data.length === 0) return null;

  const maxValue = Math.max(...data.map(d => d.value), 1);
  const chartHeight = height ?? data.length * (BAR_HEIGHT + BAR_GAP) + PADDING_TOP + PADDING_BOTTOM;
  const barAreaWidth = `calc(100% - ${PADDING_LEFT + PADDING_RIGHT}px)`;

  return (
    <svg
      role="img"
      aria-label={`Bar chart with ${data.length} items`}
      width="100%"
      height={chartHeight}
      className="block"
    >
      {data.map((item, i) => {
        const y = PADDING_TOP + i * (BAR_HEIGHT + BAR_GAP);
        const widthPercent = (item.value / maxValue) * 100;

        return (
          <g key={item.label}>
            {/* Grid line */}
            <line
              x1={PADDING_LEFT}
              y1={y + BAR_HEIGHT / 2}
              x2="100%"
              y2={y + BAR_HEIGHT / 2}
              stroke="var(--theme-border)"
              strokeWidth={1}
              strokeDasharray="4 4"
            />

            {/* Label */}
            <text
              x={PADDING_LEFT - 8}
              y={y + BAR_HEIGHT / 2}
              textAnchor="end"
              dominantBaseline="central"
              className="fill-surface-600 dark:fill-surface-400"
              fontSize={13}
              fontWeight={500}
            >
              {item.label}
            </text>

            {/* Bar — use a foreignObject so we can use CSS calc for responsive width */}
            <foreignObject x={PADDING_LEFT} y={y} width={barAreaWidth} height={BAR_HEIGHT}>
              <div
                style={{
                  width: `${widthPercent}%`,
                  height: BAR_HEIGHT,
                  background: color,
                  borderRadius: '0 4px 4px 0',
                  minWidth: item.value > 0 ? 4 : 0,
                  transition: 'width 0.3s ease',
                }}
              />
            </foreignObject>

            {/* Value */}
            <text
              x="100%"
              y={y + BAR_HEIGHT / 2}
              dx={-8}
              textAnchor="end"
              dominantBaseline="central"
              className="fill-surface-700 dark:fill-surface-300"
              fontSize={13}
              fontWeight={600}
            >
              {item.value}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
