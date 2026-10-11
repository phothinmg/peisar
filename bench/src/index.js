const crypto = require("node:crypto");
const { Bench } = require("tinybench");

const MarkdownIt = require("markdown-it");
const showdown = require("showdown");
const { marked } = require("marked");
const { Peisar } = require("peisar");

const BASE_MARKDOWN = `# Benchmark document

Peisar renders **Markdown** with [links](https://example.com), ~~deletions~~,
and \`inline code\`.

> A blockquote with a second line
> to exercise multiline parsing.

## Tasks

- [x] Parse blocks
- [ ] Render HTML
- Nested item
  - Child item

| Parser | Language | Runtime |
| ------ | -------- | ------- |
| Peisar | Rust | Node.js |
| marked | JavaScript | Node.js |

\`\`\`js
export function greet(name) {
  return \`Hello, \${name}!\`;
}
\`\`\`

---

Final paragraph with an autolink: https://example.com/docs.
`;

const markdownIt = new MarkdownIt({ html: true, linkify: true });
const showdownConverter = new showdown.Converter({
  excludeTrailingPunctuationFromURLs: true,
  simplifiedAutoLink: true,
  tables: true,
  strikethrough: true,
  tasklists: true,
});

const BENCH_OPTIONS = { time: 1_000, warmupTime: 250 };
const MEMORY_BATCHES = 7;

const renderers = [
  {
    name: "Peisar",
    render(markdown) {
      return new Peisar(markdown, { fragment: true }).html;
    },
  },
  {
    name: "markdown-it",
    render(markdown) {
      return markdownIt.render(markdown);
    },
  },
  {
    name: "marked",
    render(markdown) {
      return marked.parse(markdown);
    },
  },
  {
    name: "showdown",
    render(markdown) {
      return showdownConverter.makeHtml(markdown);
    },
  },
];

const validationChecks = [
  {
    label: "heading",
    test: (html) => /<h1\b[^>]*>Benchmark document<\/h1>/i.test(html),
  },
  { label: "strong", test: (html) => /<strong>Markdown<\/strong>/i.test(html) },
  {
    label: "link",
    test: (html) => /href="https:\/\/example\.com"/i.test(html),
  },
  { label: "blockquote", test: (html) => /<blockquote>/i.test(html) },
  {
    label: "task list",
    test: (html) => /Render HTML/i.test(html) && /Parse blocks/i.test(html),
  },
  { label: "table", test: (html) => /<table>/i.test(html) },
  {
    label: "code block",
    test: (html) => /export function greet\(name\)/i.test(html),
  },
  { label: "horizontal rule", test: (html) => /<hr\b/i.test(html) },
  {
    label: "autolink",
    test: (html) => /href="https:\/\/example\.com\/docs"/i.test(html),
  },
];

function buildDocument(multiplier) {
  return Array.from({ length: multiplier }, (_, index) => {
    return `${BASE_MARKDOWN}\n\n## Repetition ${index + 1}\n\nRepeated section ${index + 1}.`;
  }).join("\n\n");
}

const documents = [
  { name: "base", markdown: BASE_MARKDOWN },
  { name: "medium-x8", markdown: buildDocument(8) },
  { name: "large-x32", markdown: buildDocument(32) },
];

function average(values) {
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 0) {
    return (sorted[middle - 1] + sorted[middle]) / 2;
  }
  return sorted[middle];
}

function formatBytes(bytes) {
  const sign = bytes < 0 ? "-" : "";
  const absolute = Math.abs(bytes);
  if (absolute < 1024) {
    return `${sign}${absolute} B`;
  }
  if (absolute < 1024 * 1024) {
    return `${sign}${(absolute / 1024).toFixed(1)} KiB`;
  }
  return `${sign}${(absolute / (1024 * 1024)).toFixed(2)} MiB`;
}

function digest(value) {
  return crypto.createHash("sha256").update(value).digest("hex").slice(0, 12);
}

function memoryIterationsFor(markdown) {
  const bytes = Buffer.byteLength(markdown);
  if (bytes < 1_500) {
    return 4_000;
  }
  if (bytes < 12_000) {
    return 750;
  }
  return 150;
}

function forceGc() {
  if (typeof global.gc === "function") {
    global.gc();
  }
}

function validateDocument(document) {
  return renderers.map((renderer) => {
    const html = renderer.render(document.markdown);
    if (typeof html !== "string" || html.length === 0) {
      throw new Error(
        `${renderer.name} returned an empty or non-string HTML result.`,
      );
    }

    const failedChecks = validationChecks
      .filter((check) => !check.test(html))
      .map((check) => check.label);

    return {
      Renderer: renderer.name,
      Valid: failedChecks.length === 0 ? "yes" : "no",
      "Checks passed": `${validationChecks.length - failedChecks.length}/${validationChecks.length}`,
      "HTML bytes": Buffer.byteLength(html),
      Digest: digest(html),
      Notes:
        failedChecks.length === 0
          ? "All required structures found"
          : `Missing: ${failedChecks.join(", ")}`,
    };
  });
}

function measureMemory(document, renderer) {
  const iterations = memoryIterationsFor(document.markdown);
  const heapRetainedSamples = [];
  const heapPeakSamples = [];
  const rssPeakSamples = [];
  const stride = Math.max(1, Math.floor(iterations / 10));

  for (let batch = 0; batch < MEMORY_BATCHES; batch += 1) {
    forceGc();
    const before = process.memoryUsage();
    let peakHeapUsed = before.heapUsed;
    let peakRss = before.rss;

    for (let iteration = 0; iteration < iterations; iteration += 1) {
      const html = renderer.render(document.markdown);
      if (typeof html !== "string" || html.length === 0) {
        throw new Error(
          `${renderer.name} produced invalid HTML during memory profiling.`,
        );
      }

      if ((iteration + 1) % stride === 0 || iteration === iterations - 1) {
        const current = process.memoryUsage();
        peakHeapUsed = Math.max(peakHeapUsed, current.heapUsed);
        peakRss = Math.max(peakRss, current.rss);
      }
    }

    forceGc();
    const after = process.memoryUsage();
    heapRetainedSamples.push(after.heapUsed - before.heapUsed);
    heapPeakSamples.push(peakHeapUsed - before.heapUsed);
    rssPeakSamples.push(peakRss - before.rss);
  }

  return {
    Renderer: renderer.name,
    Iterations: iterations,
    Batches: MEMORY_BATCHES,
    "Heap retained avg": formatBytes(Math.round(average(heapRetainedSamples))),
    "Heap retained med": formatBytes(Math.round(median(heapRetainedSamples))),
    "Heap peak avg": formatBytes(Math.round(average(heapPeakSamples))),
    "Heap peak med": formatBytes(Math.round(median(heapPeakSamples))),
    "RSS peak avg": formatBytes(Math.round(average(rssPeakSamples))),
  };
}

async function benchmarkDocument(document) {
  const bench = new Bench(BENCH_OPTIONS);
  for (const renderer of renderers) {
    bench.add(renderer.name, () => renderer.render(document.markdown));
  }
  await bench.run();
  return bench.table();
}

async function main() {
  console.log(
    "Markdown benchmark with output validation, speed, and memory profiling.",
  );
  console.log(
    `Speed uses tinybench (${BENCH_OPTIONS.time} ms task time, ${BENCH_OPTIONS.warmupTime} ms warmup).`,
  );
  if (typeof global.gc !== "function") {
    console.log("Tip: run with --expose-gc for steadier memory numbers.");
  }

  for (const document of documents) {
    console.log("");
    console.log(
      `Scenario: ${document.name} (${Buffer.byteLength(document.markdown)} bytes of Markdown).`,
    );

    const validationRows = validateDocument(document);
    console.log("Output validation");
    console.table(validationRows);

    const invalidRows = validationRows.filter((row) => row.Valid !== "yes");
    if (invalidRows.length > 0) {
      throw new Error(
        `Validation failed for scenario "${document.name}". Refusing to benchmark invalid output.`,
      );
    }

    console.log("Speed");
    console.table(await benchmarkDocument(document));

    console.log("Memory");
    console.table(
      renderers.map((renderer) => measureMemory(document, renderer)),
    );
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
