// Lightweight, dependency-free SVG charts for the dashboard. They scale to the
// container width via a fixed viewBox.

export type Series = { color: string; values: number[] };

const GRID = "#eef0f6";
const AXIS = "#9aa0b4";

function gridlines(padX: number, W: number, padTop: number, innerH: number) {
  return [0, 0.25, 0.5, 0.75, 1].map((g) => {
    const y = padTop + innerH * (1 - g);
    return (
      <line
        key={g}
        x1={padX}
        x2={W - padX}
        y1={y}
        y2={y}
        stroke={GRID}
        strokeWidth={1}
      />
    );
  });
}

/** Stacked bar chart. */
export function BarChart({
  labels,
  series,
  height = 220,
}: {
  labels: string[];
  series: Series[];
  height?: number;
}) {
  const W = 720;
  const H = height;
  const padX = 10;
  const padTop = 12;
  const padBottom = 26;
  const innerH = H - padTop - padBottom;
  const n = labels.length || 1;
  const slot = (W - padX * 2) / n;
  const barW = Math.min(30, slot * 0.5);
  const totals = labels.map((_, i) =>
    series.reduce((s, ser) => s + (ser.values[i] ?? 0), 0),
  );
  const max = Math.max(1, ...totals);

  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-hidden>
      {gridlines(padX, W, padTop, innerH)}
      {labels.map((lab, i) => {
        const x = padX + slot * i + slot / 2 - barW / 2;
        let cursor = padTop + innerH;
        return (
          <g key={lab}>
            {series.map((ser, si) => {
              const h = ((ser.values[i] ?? 0) / max) * innerH;
              cursor -= h;
              return (
                <rect
                  key={si}
                  x={x}
                  y={cursor}
                  width={barW}
                  height={Math.max(0, h)}
                  rx={4}
                  fill={ser.color}
                />
              );
            })}
            <text
              x={padX + slot * i + slot / 2}
              y={H - 8}
              textAnchor="middle"
              fontSize="11"
              fill={AXIS}
            >
              {lab}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

/** Multi-series line chart with soft area fill. */
export function LineChart({
  labels,
  series,
  height = 230,
}: {
  labels: string[];
  series: Series[];
  height?: number;
}) {
  const W = 720;
  const H = height;
  const padX = 12;
  const padTop = 14;
  const padBottom = 26;
  const innerH = H - padTop - padBottom;
  const n = labels.length || 1;
  const max = Math.max(1, ...series.flatMap((s) => s.values));
  const xAt = (i: number) =>
    padX + (W - padX * 2) * (n === 1 ? 0 : i / (n - 1));
  const yAt = (v: number) => padTop + innerH * (1 - v / max);
  const baseY = padTop + innerH;

  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-hidden>
      {gridlines(padX, W, padTop, innerH)}
      {series.map((ser, si) => {
        const line = ser.values.map((v, i) => `${xAt(i)},${yAt(v)}`).join(" ");
        const area = `${xAt(0)},${baseY} ${line} ${xAt(ser.values.length - 1)},${baseY}`;
        return (
          <g key={si}>
            <polygon points={area} fill={ser.color} opacity={0.08} />
            <polyline
              points={line}
              fill="none"
              stroke={ser.color}
              strokeWidth={2.5}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
            {ser.values.map((v, i) => (
              <circle
                key={i}
                cx={xAt(i)}
                cy={yAt(v)}
                r={3}
                fill="#fff"
                stroke={ser.color}
                strokeWidth={2}
              />
            ))}
          </g>
        );
      })}
      {labels.map((lab, i) => (
        <text
          key={lab}
          x={xAt(i)}
          y={H - 8}
          textAnchor="middle"
          fontSize="11"
          fill={AXIS}
        >
          {lab}
        </text>
      ))}
    </svg>
  );
}

/** Semicircular gauge (0..1). */
export function Gauge({ value }: { value: number }) {
  const W = 220;
  const H = 124;
  const cx = W / 2;
  const cy = 112;
  const r = 90;
  const stroke = 16;
  const circ = Math.PI * r;
  const dash = circ * Math.min(1, Math.max(0, value));
  const path = `M ${cx - r} ${cy} A ${r} ${r} 0 0 1 ${cx + r} ${cy}`;
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-hidden>
      <defs>
        <linearGradient id="gauge-grad" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#3c50e0" />
          <stop offset="100%" stopColor="#80caee" />
        </linearGradient>
      </defs>
      <path d={path} fill="none" stroke={GRID} strokeWidth={stroke} strokeLinecap="round" />
      <path
        d={path}
        fill="none"
        stroke="url(#gauge-grad)"
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={`${dash} ${circ}`}
      />
    </svg>
  );
}
