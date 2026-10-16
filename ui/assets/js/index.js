
/* It popover functionality */
function registerItPopover(el) {
  el.querySelector("#rename-it").addEventListener("click", () => {
    el.dispatchEvent(new Event("closepopover"));
  });
  el.querySelector("#delete-it").addEventListener("click", () => {
    el.dispatchEvent(new Event("closepopover"));
  });
}

htmx.onLoad((el) => {
  if (el.hasAttribute("data-it-popover")) {
    registerItPopover(el);
  }
});
document.body.addEventListener("htmx:beforeSwap", (ev) => {
  if (ev.detail.xhr.status >= 300) {
      ev.detail.shouldSwap = true;
  }
});
