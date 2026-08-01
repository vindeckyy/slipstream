import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import GithubSlugger from "github-slugger";
import { toString } from "mdast-util-to-string";
import remarkMdx from "remark-mdx";
import remarkParse from "remark-parse";
import { unified } from "unified";
import { visit } from "unist-util-visit";

const repoRoot = fileURLToPath(new URL("../../", import.meta.url));
const docsRoot = join(repoRoot, "docs-site", "content", "docs");
const skippedSegments = new Set(["vendor"]);

const files = execFileSync(
  "git",
  ["ls-files", "-z", "--", "*.md", "*.mdx"],
  { cwd: repoRoot, encoding: "utf8" },
)
  .split("\0")
  .filter(Boolean)
  .filter((file) => !file.split("/").some((part) => skippedSegments.has(part)));

const issues = [];
const documentCache = new Map();
const anchorCache = new Map();

function report(file, line, message) {
  issues.push(`${file}:${line}: ${message}`);
}

function decode(value, file, line, target) {
  try {
    return decodeURIComponent(value);
  } catch {
    report(file, line, `malformed link target "${target}"`);
    return null;
  }
}

function documentFor(file) {
  if (documentCache.has(file)) return documentCache.get(file);

  const source = readFileSync(file, "utf8");
  const parser = unified().use(remarkParse);
  if (extname(file).toLowerCase() === ".mdx") parser.use(remarkMdx);
  const document = { source, tree: parser.parse(source) };
  documentCache.set(file, document);
  return document;
}

function addHtmlIds(anchors, value) {
  for (const match of value.matchAll(/\bid\s*=\s*["']([^"']+)["']/g)) {
    anchors.add(match[1]);
  }
}

function anchorsFor(file) {
  if (anchorCache.has(file)) return anchorCache.get(file);

  const anchors = new Set();
  const slugger = new GithubSlugger();
  const { tree } = documentFor(file);
  const usesFumadocsIds = file.startsWith(`${docsRoot}${sep}`);

  visit(tree, (node) => {
    if (node.type === "html") addHtmlIds(anchors, node.value);

    if (node.type === "mdxJsxFlowElement" || node.type === "mdxJsxTextElement") {
      for (const attribute of node.attributes ?? []) {
        if (attribute.type === "mdxJsxAttribute" && attribute.name === "id" && typeof attribute.value === "string") {
          anchors.add(attribute.value);
        }
      }
    }

    if (node.type !== "heading") return;
    const lastChild = node.children.at(-1);
    const explicit =
      usesFumadocsIds && lastChild?.type === "text" ? lastChild.value.match(/\s*\[#(.+?)]\s*$/) : null;
    if (explicit) {
      anchors.add(explicit[1]);
      return;
    }

    anchors.add(slugger.slug(toString(node)));
  });

  anchorCache.set(file, anchors);
  return anchors;
}

function markdownFileForDirectory(directory) {
  for (const name of ["README.md", "index.md", "index.mdx"]) {
    const candidate = join(directory, name);
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

function docsRouteFile(route) {
  if (route === "/docs" || route === "/docs/") {
    return join(docsRoot, "index.mdx");
  }

  if (!route.startsWith("/docs/")) return null;
  const name = route.slice("/docs/".length).replace(/\/$/, "");
  for (const extension of [".md", ".mdx"]) {
    const candidate = join(docsRoot, `${name}${extension}`);
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

function resolveTarget(sourceFile, pathname) {
  if (pathname.startsWith("/")) {
    const docsFile = docsRouteFile(pathname);
    if (docsFile) return docsFile;
    if (pathname === "/") return join(repoRoot, "docs-site", "src", "routes", "index.tsx");
    if (pathname === "/api" || pathname === "/api/") {
      return join(repoRoot, "docs-site", "src", "routes", "api", "index.tsx");
    }
    return join(repoRoot, "docs-site", "public", pathname.slice(1));
  }

  return resolve(dirname(join(repoRoot, sourceFile)), pathname);
}

function checkTarget(sourceFile, line, rawTarget) {
  let target = rawTarget.trim().replace(/^<|>$/g, "");
  if (!target) return;
  if (!target.startsWith("#") && /^[a-z][a-z0-9+.-]*:/i.test(target)) return;
  if (target.startsWith("//") || target.startsWith("{")) return;

  const hashAt = target.indexOf("#");
  const rawFragment = hashAt === -1 ? "" : target.slice(hashAt + 1);
  if (hashAt !== -1) target = target.slice(0, hashAt);
  const queryAt = target.indexOf("?");
  if (queryAt !== -1) target = target.slice(0, queryAt);

  const pathname = decode(target, sourceFile, line, rawTarget);
  const fragment = decode(rawFragment, sourceFile, line, rawTarget);
  if (pathname === null || fragment === null) return;

  let resolved = pathname ? resolveTarget(sourceFile, pathname) : join(repoRoot, sourceFile);
  if (!existsSync(resolved)) {
    report(sourceFile, line, `broken local link "${rawTarget}"`);
    return;
  }

  if (statSync(resolved).isDirectory()) {
    const index = markdownFileForDirectory(resolved);
    if (!fragment) return;
    if (!index) {
      report(sourceFile, line, `cannot check heading "#${fragment}" because the directory has no Markdown index`);
      return;
    }
    resolved = index;
  }

  if (!fragment || ![".md", ".mdx"].includes(extname(resolved).toLowerCase())) return;
  if (!anchorsFor(resolved).has(fragment)) {
    report(sourceFile, line, `missing heading "#${fragment}" in "${rawTarget}"`);
  }
}

function lineForMatch(source, nodeLine, index) {
  return nodeLine + source.slice(0, index).split("\n").length - 1;
}

for (const file of files) {
  const absoluteFile = join(repoRoot, file);
  let document;
  try {
    document = documentFor(absoluteFile);
  } catch (error) {
    report(file, 1, `could not parse Markdown: ${error.message}`);
    continue;
  }

  const definitions = new Set();
  visit(document.tree, (node) => {
    if (node.type === "definition") definitions.add(node.identifier.toLowerCase());
  });

  visit(document.tree, (node) => {
    const line = node.position?.start.line ?? 1;
    if (node.type === "link" || node.type === "image" || node.type === "definition") {
      checkTarget(file, line, node.url);
      return;
    }

    if (node.type === "linkReference" || node.type === "imageReference") {
      if (!definitions.has(node.identifier.toLowerCase())) {
        report(file, line, `undefined link reference "${node.identifier}"`);
      }
      return;
    }

    if (node.type === "html") {
      for (const match of node.value.matchAll(/\b(?:href|src)\s*=\s*(["'])(.*?)\1/gs)) {
        checkTarget(file, lineForMatch(node.value, line, match.index), match[2]);
      }
      return;
    }

    if (node.type === "mdxJsxFlowElement" || node.type === "mdxJsxTextElement") {
      for (const attribute of node.attributes ?? []) {
        if (
          attribute.type === "mdxJsxAttribute" &&
          (attribute.name === "href" || attribute.name === "src") &&
          typeof attribute.value === "string"
        ) {
          checkTarget(file, attribute.position?.start.line ?? line, attribute.value);
        }
      }
    }
  });
}

if (issues.length > 0) {
  for (const issue of issues) console.error(issue);
  console.error(`\n${issues.length} broken documentation link${issues.length === 1 ? "" : "s"}`);
  process.exitCode = 1;
} else {
  console.log(`Checked ${files.length} Markdown files; all local links resolve.`);
}
