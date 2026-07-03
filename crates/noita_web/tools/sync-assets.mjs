import { copyFile, mkdir, open, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ASSET_DIRS = [
    ["ui_gfx/gun_actions", "gun_actions"],
    ["ui_gfx/inventory", "inventory"],
    ["items_gfx/wands", "wands"],
];

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(webRoot, "..", "..");
const dataRoot = process.env.NOITA_DATA_PATH
    ? path.resolve(process.env.NOITA_DATA_PATH)
    : path.resolve(repoRoot, "noita-data");
const fallbackDataRoot = path.resolve(repoRoot, "crates", "noita_sim", "data");
const destRoot = path.join(webRoot, "public", "assets");

async function existingDataRoot() {
    for (const candidate of [dataRoot, fallbackDataRoot]) {
        try {
            if ((await stat(candidate)).isDirectory()) {
                return candidate;
            }
        } catch (error) {
            if (error.code !== "ENOENT") {
                throw error;
            }
        }
    }
    throw new Error(
        "Noita data not found: set NOITA_DATA_PATH or init the noita-data submodule",
    );
}

async function isUpToDate(src, dest) {
    try {
        const [srcMeta, destMeta] = await Promise.all([stat(src), stat(dest)]);
        return srcMeta.size === destMeta.size;
    } catch (error) {
        if (error.code === "ENOENT") {
            return false;
        }
        throw error;
    }
}

async function mirrorPngs(srcDir, destDir) {
    await mkdir(destDir, { recursive: true });
    const entries = (await readdir(srcDir, { withFileTypes: true }))
        .filter(
            (entry) => entry.isFile() && path.extname(entry.name) === ".png",
        )
        .sort((left, right) => left.name.localeCompare(right.name));

    let copied = 0;
    for (const entry of entries) {
        const src = path.join(srcDir, entry.name);
        const dest = path.join(destDir, entry.name);
        if (await isUpToDate(src, dest)) {
            continue;
        }
        await copyFile(src, dest);
        copied += 1;
    }
    return copied;
}

const sourceRoot = await existingDataRoot();
let copied = 0;
for (const [inner, dest] of ASSET_DIRS) {
    const srcDir = path.join(sourceRoot, inner);
    if (!(await stat(srcDir)).isDirectory()) {
        throw new Error(`Noita asset directory not found: ${srcDir}`);
    }
    copied += await mirrorPngs(srcDir, path.join(destRoot, dest));
}

console.log(
    `Synced ${copied} Noita asset(s) into ${path.relative(webRoot, destRoot)}.`,
);
