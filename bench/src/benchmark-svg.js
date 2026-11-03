const fs = require("node:fs");
const path = require("node:path");
const { version } = require("../../package.json");

const COLORS = ["#0969da", "#1a7f37", "#bf8700", "#cf222e", "#8250df"];
const OUTPUT_PATH = path.resolve(__dirname, "../../docs/benchmark-results.svg");

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function niceScale(maxValue) {
  const target = Math.max(maxValue * 1.1, 1);
  const magnitude = 10 ** Math.floor(Math.log10(target));
  const fraction = target / magnitude;
  const step = [1, 2, 2.5, 5, 10].find((candidate) => candidate >= fraction);
  return step * magnitude;
}

function formatOperations(value) {
  return Math.round(value).toLocaleString("en-US");
}

function renderPanel({
  title,
  subtitle,
  results,
  valueFormatter,
  scaleFormatter,
  titleY,
  subtitleY,
  plotTop,
  plotBottom,
}) {
  const scenarioCenters = [190, 570, 950];
  const plotHeight = plotBottom - plotTop;
  const entries = results[0].values;
  const bars = [];
  const axes = [];
  const labels = [];

  results.forEach((scenario, scenarioIndex) => {
    const center = scenarioCenters[scenarioIndex];
    const scale = niceScale(Math.max(...scenario.values.map((entry) => entry.value)));
    const baseline = plotBottom;
    const groupWidth = entries.length * 28 + (entries.length - 1) * 7;
    const firstX = center - groupWidth / 2;
    const scenarioLabel = scenario.name
      .replace(/-x\d+$/, "")
      .replace(/^./, (letter) => letter.toUpperCase());

    labels.push(
      `<text x="${center}" y="${plotTop - 36}" text-anchor="middle" class="category">${escapeXml(scenarioLabel)}</text>`,
      `<text x="${center}" y="${plotTop - 17}" text-anchor="middle" class="scale">Scale: 0 - ${scaleFormatter(scale)}</text>`,
    );
    axes.push(
      `<line x1="${center - 145}" y1="${baseline}" x2="${center + 145}" y2="${baseline}" class="axis"/>`,
      `<line x1="${center - 145}" y1="${plotTop}" x2="${center + 145}" y2="${plotTop}" class="grid"/>`,
      `<line x1="${center - 145}" y1="${plotTop + plotHeight / 2}" x2="${center + 145}" y2="${plotTop + plotHeight / 2}" class="grid"/>`,
    );

    scenario.values.forEach((entry, rendererIndex) => {
      const barHeight = (entry.value / scale) * plotHeight;
      const x = firstX + rendererIndex * 35;
      const y = baseline - barHeight;
      bars.push(
        `<rect x="${x}" y="${y.toFixed(1)}" width="28" height="${Math.max(barHeight, 0.5).toFixed(1)}" rx="2" fill="${COLORS[rendererIndex % COLORS.length]}"/>`,
        `<text x="${x + 14}" y="${Math.max(plotTop + 10, y - 5).toFixed(1)}" text-anchor="middle" class="value">${valueFormatter(entry.value)}</text>`,
      );
    });
  });

  return [
    `<text x="36" y="${titleY}" class="panel-title">${escapeXml(title)}</text>`,
    `<text x="36" y="${subtitleY}" class="subtitle">${escapeXml(subtitle)}</text>`,
    ...labels,
    ...axes,
    ...bars,
  ].join("\n");
}

function createSvg(results) {
  if (results.length !== 3) {
    throw new Error("Benchmark SVG generation expects exactly three scenarios.");
  }

  const rendererNames = results[0].throughput.map((entry) => entry.renderer);
  if (rendererNames.length === 0) {
    throw new Error("Benchmark SVG generation requires renderer measurements.");
  }

  for (const result of results) {
    for (const key of ["throughput", "peakHeapMiB"]) {
      const entries = result[key];
      if (
        entries.length !== rendererNames.length ||
        entries.some(
          (entry, index) =>
            entry.renderer !== rendererNames[index] ||
            !Number.isFinite(entry.value) ||
            entry.value < 0,
        )
      ) {
        throw new Error(`Invalid ${key} measurements for "${result.name}".`);
      }
    }
  }

  const legends = rendererNames.flatMap((name, index) => {
    const x = 64 + index * 210;
    return [
      `<rect x="${x}" y="78" width="14" height="14" rx="2" fill="${COLORS[index % COLORS.length]}"/>`,
      `<text x="${x + 21}" y="90" class="legend">${escapeXml(name)}</text>`,
    ];
  });
  const throughput = renderPanel({
    title: "Average throughput (operations/s)",
    subtitle: "Higher is better - independent scale per document size",
    results: results.map(({ name, throughput }) => ({
      name,
      values: throughput,
    })),
    valueFormatter: formatOperations,
    scaleFormatter: formatOperations,
    titleY: 126,
    subtitleY: 145,
    plotTop: 207,
    plotBottom: 345,
  });
  const memory = renderPanel({
    title: "Average peak heap (MiB)",
    subtitle: "Lower is better - seven batches - independent scale per document size",
    results: results.map(({ name, peakHeapMiB }) => ({
      name,
      values: peakHeapMiB,
    })),
    valueFormatter: (value) => value.toFixed(2),
    scaleFormatter: (value) => `${value.toFixed(1)} MiB`,
    titleY: 414,
    subtitleY: 433,
    plotTop: 495,
    plotBottom: 633,
  });

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1140 700" role="img" aria-labelledby="title description">
  <title id="title">Markdown renderer benchmark results</title>
  <desc id="description">Benchmark charts compare rendering throughput and average peak heap in mebibytes for ${escapeXml(rendererNames.join(", "))} across three Markdown document sizes. Values are generated by the current benchmark run.</desc>
  <style>
    text { fill: #24292f; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .heading { font-size: 22px; font-weight: 700; }
    .subtitle { fill: #57606a; font-size: 13px; }
    .panel-title { font-size: 16px; font-weight: 600; }
    .scale { fill: #57606a; font-size: 11px; }
    .value { font-size: 10px; }
    .category { font-size: 12px; font-weight: 600; }
    .legend { font-size: 13px; }
    .axis { stroke: #8c959f; stroke-width: 1; }
    .grid { stroke: #d8dee4; stroke-width: 1; }
  </style>
  <rect width="1140" height="700" rx="12" fill="#ffffff"/>
  <text x="36" y="38" class="heading">Markdown renderer benchmarks</text>
  <text x="36" y="61" class="subtitle">Peisar ${escapeXml(version)} - Node.js ${escapeXml(process.version)} - measured on this run</text>
  ${legends.join("\n  ")}
  ${throughput}
  <line x1="380" y1="154" x2="380" y2="362" class="grid"/>
  <line x1="760" y1="154" x2="760" y2="362" class="grid"/>
  <line x1="36" y1="382" x2="1104" y2="382" class="grid"/>
  ${memory}
  <line x1="380" y1="478" x2="380" y2="650" class="grid"/>
  <line x1="760" y1="478" x2="760" y2="650" class="grid"/>
</svg>`;
}

function writeBenchmarkSvg(results) {
  const svg = createSvg(results);
  fs.mkdirSync(path.dirname(OUTPUT_PATH), { recursive: true });
  fs.writeFileSync(OUTPUT_PATH, `${svg}\n`, "utf8");
  return OUTPUT_PATH;
}

module.exports = { writeBenchmarkSvg };
