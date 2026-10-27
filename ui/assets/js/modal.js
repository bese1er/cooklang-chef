function handleDirectoryModal() {
  const directoryModal = document.querySelector("#directory-modal");
  if (directoryModal) {
    document.querySelector("#open-directory-modal").addEventListener("click", () => {
      directoryModal.showModal();
    });
    document.querySelector("#close-directory-modal").addEventListener("click", () => {
      directoryModal.close();
    });
    directoryModal.addEventListener("click", (event) => {
      if (event.target === directoryModal) {
        directoryModal.close();
      }
    });
  }
}

function handleRecipeModal() {
  const recipeModal = document.querySelector("#recipe-modal");
  if (recipeModal) {
    document.querySelector("#open-recipe-modal").addEventListener("click", () => {
      recipeModal.showModal();
    });
    document.querySelector("#close-recipe-modal").addEventListener("click", () => {
      recipeModal.close();
    });
    recipeModal.addEventListener("click", (event) => {
      if (event.target === recipeModal) {
        recipeModal.close();
      }
    });
  }
}

handleDirectoryModal()
handleRecipeModal()
document.body.addEventListener("htmx:afterOnLoad", (ev) => {
  if (ev.detail.elt.id === "hot-reload-target") {
    handleDirectoryModal()
    handleRecipeModal()
  }
});

