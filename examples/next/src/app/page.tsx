import { Peisar } from "peisar";
import fs from "node:fs";
const content = fs.readFileSync("README.md", "utf8");
const p = new Peisar(content, { fragment: true });

const rawHtml = p.html;
export default function Home() {
  return (
   <div dangerouslySetInnerHTML={{ __html: rawHtml }} />
  );
}
