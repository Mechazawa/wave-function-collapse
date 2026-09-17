import init, { Solver, randomSeed } from "../pkg/wfc_wasm";
import { COAST, renderAtlas, tilesFromEdges } from "./tiles";

const TILE_PIXELS = 24;

function element<T extends HTMLElement>(id: string): T {
    const found = document.getElementById(id);

    if (found === null) {
        throw new Error(`the page has no #${id}`);
    }

    return found as T;
}

await init();

const atlas = renderAtlas(COAST, TILE_PIXELS);
const definitions = tilesFromEdges(COAST);

const canvas = element<HTMLCanvasElement>("board");
const maybeContext = canvas.getContext("2d", { alpha: false });

if (maybeContext === null) {
    throw new Error("this browser gave us no 2d canvas context");
}

// Reassigned so the narrowing survives into the closures below.
const context = maybeContext;

const widthInput = element<HTMLInputElement>("width");
const heightInput = element<HTMLInputElement>("height");
const budgetInput = element<HTMLInputElement>("budget");
const seedInput = element<HTMLInputElement>("seed");
const readout = element<HTMLElement>("readout");

let solver: Solver | null = null;
let frame: number | null = null;
let frames = 0;
let framesSince = performance.now();
let rate = 0;

/** Redraws only what the last step changed, reading the grid straight back. */
function draw(current: Solver): void {
    const cells = current.cells();
    const entropy = current.entropy();
    const columns = current.gridWidth;
    const base = current.baseEntropy;

    for (const cell of current.dirtyCells()) {
        const x = (cell % columns) * TILE_PIXELS;
        const y = Math.floor(cell / columns) * TILE_PIXELS;
        const tile = cells[cell] ?? -1;

        if (tile < 0) {
            // Lighter the further a cell has narrowed down.
            const settled = 1 - (entropy[cell] ?? base) / base;
            const shade = Math.round(34 + settled * 90);

            context.fillStyle = `rgb(${shade} ${shade} ${shade + 12})`;
            context.fillRect(x, y, TILE_PIXELS, TILE_PIXELS);
            continue;
        }

        const drawn = atlas[tile];

        if (drawn !== undefined) {
            context.drawImage(drawn, x, y);
        }
    }
}

function report(current: Solver): void {
    const stuck = !current.done && current.lastStepCount === 0;
    const state = current.done ? "solved" : stuck ? "stuck" : "solving";

    readout.textContent = [
        `${current.collapsed} / ${current.total} cells`,
        `${Math.round(current.progress * 100)}%`,
        `${rate.toFixed(0)} fps`,
        `seed ${current.seed}`,
        state,
    ].join("   ");
}

function measureRate(): void {
    frames += 1;
    const elapsed = performance.now() - framesSince;

    if (elapsed >= 500) {
        rate = (frames * 1000) / elapsed;
        frames = 0;
        framesSince = performance.now();
    }
}

function tick(): void {
    if (solver === null) {
        return;
    }

    const done = solver.stepFor(Number(budgetInput.value));

    draw(solver);
    measureRate();
    report(solver);

    // A step that got nowhere while the wave is unfinished means no cell can be
    // collapsed, so another frame would only spin.
    frame = done || solver.lastStepCount === 0 ? null : requestAnimationFrame(tick);
}

function pause(): void {
    if (frame !== null) {
        cancelAnimationFrame(frame);
        frame = null;
    }
}

function start(seed: bigint): void {
    pause();
    solver?.free();

    const width = Math.max(1, Number(widthInput.value));
    const height = Math.max(1, Number(heightInput.value));

    solver = new Solver(definitions, width, height, seed);
    seedInput.value = seed.toString();

    canvas.width = width * TILE_PIXELS;
    canvas.height = height * TILE_PIXELS;

    frames = 0;
    framesSince = performance.now();

    draw(solver);
    report(solver);

    frame = requestAnimationFrame(tick);
}

element<HTMLButtonElement>("generate").addEventListener("click", () => {
    const typed = seedInput.value.trim();

    start(typed === "" ? randomSeed() : BigInt(typed));
});

element<HTMLButtonElement>("reroll").addEventListener("click", () => start(randomSeed()));
element<HTMLButtonElement>("stop").addEventListener("click", pause);

element<HTMLButtonElement>("once").addEventListener("click", () => {
    if (solver === null) {
        return;
    }

    pause();
    solver.step(1);
    draw(solver);
    report(solver);
});

start(randomSeed());
