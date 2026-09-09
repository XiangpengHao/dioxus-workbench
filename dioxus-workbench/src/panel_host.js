// Dioxus document bridge: the VDOM owns each host permanently. Only its box
// follows the content slot, so structural docking never reparents content.
const host = document.getElementById(hostId);
const slot = document.getElementById(slotId);
const root = document.getElementById(rootId);
if (!host || !slot || !root) return;
let frame = 0;
let previous = "";
let stopped = false;
const measure = () => {
    frame = 0;
    if (stopped) return;
    const box = slot.getBoundingClientRect();
    const origin = root.getBoundingClientRect();
    const visible = box.width > 0 && box.height > 0 && slot.getClientRects().length > 0;
    host.style.left = `${box.left - origin.left}px`;
    host.style.top = `${box.top - origin.top}px`;
    host.style.width = `${box.width}px`;
    host.style.height = `${box.height}px`;
    host.style.visibility = visible ? "visible" : "hidden";
    const next = JSON.stringify({width: visible ? box.width : 0, height: visible ? box.height : 0, visible});
    if (next !== previous) { previous = next; dioxus.send(JSON.parse(next)); }
};
const schedule = () => { if (!frame && !stopped) frame = requestAnimationFrame(measure); };
const observer = new ResizeObserver(schedule);
// Every ancestor can move the slot without resizing it (a sibling divider,
// for example). Their resize notifications cover those position changes.
for (let node = slot; node; node = node.parentElement) {
    observer.observe(node);
    if (node === root) break;
}
window.addEventListener("resize", schedule);
measure();
try { await dioxus.recv(); }
finally {
    stopped = true;
    observer.disconnect();
    window.removeEventListener("resize", schedule);
    if (frame) cancelAnimationFrame(frame);
}
