import type { Tile } from "../pkg/wfc_wasm";

/**
 * A tile before its adjacency has been worked out: how it is drawn, how often it
 * should appear, and a label per side.
 *
 * Two tiles may sit beside each other when the labels on the sides that touch are
 * equal. Anything comparable by string works; these labels name what crosses the
 * edge, so "pipe" only ever meets "pipe".
 */
export type Side = "up" | "right" | "down" | "left";

export interface EdgeTile {
    readonly up: string;
    readonly right: string;
    readonly down: string;
    readonly left: string;
    readonly weight?: number;
    /** Draws the tile into a `size` by `size` context with the origin at 0, 0. */
    readonly draw: (context: CanvasRenderingContext2D, size: number) => void;
}

/**
 * Turns edge labels into the adjacency the solver wants. The solver itself takes
 * only indices, so any rule that decides "may these two touch" can produce its
 * input; this is one such rule.
 */
export function tilesFromEdges(tiles: readonly EdgeTile[]): Tile[] {
    // `touching` is the side of the other tile that meets this label.
    const allowed = (touching: Side, label: string) =>
        tiles.flatMap((other, index) => (other[touching] === label ? [index] : []));

    return tiles.map((tile) => ({
        up: allowed("down", tile.up),
        right: allowed("left", tile.right),
        down: allowed("up", tile.down),
        left: allowed("right", tile.left),
        weight: tile.weight ?? 1,
    }));
}

/** Renders each tile once, so the render loop can blit instead of redrawing. */
export function renderAtlas(tiles: readonly EdgeTile[], size: number): HTMLCanvasElement[] {
    return tiles.map((tile) => {
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;

        const context = canvas.getContext("2d");

        if (context === null) {
            throw new Error("this browser gave us no 2d canvas context");
        }

        tile.draw(context, size);

        return canvas;
    });
}

const GRASS = "#3f8f4a";
const WATER = "#2f6fb0";
const SAND = "#d8c98a";

/**
 * Coast tiles. Land meets water only through sand, so the solver has to lay a
 * beach wherever the two would otherwise touch.
 */
export const COAST: readonly EdgeTile[] = [
    {
        up: "grass",
        right: "grass",
        down: "grass",
        left: "grass",
        weight: 6,
        draw: (context, size) => fill(context, size, GRASS),
    },
    {
        up: "water",
        right: "water",
        down: "water",
        left: "water",
        weight: 6,
        draw: (context, size) => fill(context, size, WATER),
    },
    {
        up: "grass",
        right: "sand",
        down: "sand",
        left: "grass",
        weight: 2,
        draw: (context, size) => corner(context, size, GRASS, SAND, 0),
    },
    {
        up: "grass",
        right: "grass",
        down: "sand",
        left: "sand",
        weight: 2,
        draw: (context, size) => corner(context, size, GRASS, SAND, 90),
    },
    {
        up: "sand",
        right: "grass",
        down: "grass",
        left: "sand",
        weight: 2,
        draw: (context, size) => corner(context, size, GRASS, SAND, 180),
    },
    {
        up: "sand",
        right: "sand",
        down: "grass",
        left: "grass",
        weight: 2,
        draw: (context, size) => corner(context, size, GRASS, SAND, 270),
    },
    {
        up: "sand",
        right: "sand",
        down: "sand",
        left: "sand",
        weight: 3,
        draw: (context, size) => fill(context, size, SAND),
    },
    {
        up: "sand",
        right: "water",
        down: "water",
        left: "sand",
        weight: 2,
        draw: (context, size) => corner(context, size, SAND, WATER, 0),
    },
    {
        up: "sand",
        right: "sand",
        down: "water",
        left: "water",
        weight: 2,
        draw: (context, size) => corner(context, size, SAND, WATER, 90),
    },
    {
        up: "water",
        right: "sand",
        down: "sand",
        left: "water",
        weight: 2,
        draw: (context, size) => corner(context, size, SAND, WATER, 180),
    },
    {
        up: "water",
        right: "water",
        down: "sand",
        left: "sand",
        weight: 2,
        draw: (context, size) => corner(context, size, SAND, WATER, 270),
    },
];

function fill(context: CanvasRenderingContext2D, size: number, colour: string): void {
    context.fillStyle = colour;
    context.fillRect(0, 0, size, size);
}

/** `outer` across the tile, `inner` rounded into one corner chosen by `rotation`. */
function corner(
    context: CanvasRenderingContext2D,
    size: number,
    outer: string,
    inner: string,
    rotation: number,
): void {
    fill(context, size, outer);

    context.save();
    context.translate(size / 2, size / 2);
    context.rotate((rotation * Math.PI) / 180);
    context.translate(-size / 2, -size / 2);

    context.fillStyle = inner;
    context.beginPath();
    context.moveTo(size, size);
    context.arc(size, size, size, Math.PI, Math.PI * 1.5);
    context.closePath();
    context.fill();

    context.restore();
}
