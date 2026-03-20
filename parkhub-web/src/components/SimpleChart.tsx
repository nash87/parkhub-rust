interface BarChartProps {
  data: { label: string; value: number }[];
  height?: number;
  color?: string;
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
              stroke="var(--color-surface-200)"
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
