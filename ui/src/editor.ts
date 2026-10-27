import {defineCooklangMode} from "./ace-cooklang";

declare global {
    interface Window {
        ace: any;
    }
}

let editor;

async function handleEditor(): Promise<void> {
    defineCooklangMode();
    editor = window.ace.edit("editor", {
        mode: "ace/mode/cooklang",
        wrap: true,
        showPrintMargin: false,
        fontSize: 14,
        fontFamily: "JetBrains Mono, ui-monospace, monospace",
        placeholder: "Write your recipe here",
        highlightActiveLine: false,
        tabSize: 2,
    });
    editor.renderer.setScrollMargin(12, 12);
    editor.renderer.setPadding(12);
    editor.focus();

    const saveBtn = document.getElementById("saveEditBtn") as HTMLButtonElement;
    saveBtn.disabled = true;

    function enableSaving() {
        saveBtn.disabled = false;
    }

    editor.session.on("change", enableSaving);
}

async function checkEditor(): Promise<void> {
    if (document.getElementById("editor")) {
        handleEditor();
    }
}

document.body.addEventListener("htmx:configRequest", (event: Event) => {
    const e = event as CustomEvent;

    if (e.detail.path.startsWith("/editor/")) {
        e.detail.parameters.content = editor.getValue();
    }
})

document.body.addEventListener("htmx:beforeSwap", (event: Event) => {
    const e = event as CustomEvent;

    if (e.detail.xhr.status >= 300) {
        e.detail.shouldSwap = true;
    }
})

checkEditor();
