import { api } from "./api.js";
import { initPage } from "./ticket-common.js";

const me = await initPage();
if (!me.is_admin) {
  location.href = "/";
  throw new Error("not an admin");
}

const rows = document.getElementById("tag-rows");
const form = document.getElementById("tag-form");
const error = document.getElementById("error");

async function load() {
  const tags = await api("/api/tags");
  rows.replaceChildren(...tags.map((tag) => {
    const row = document.createElement("tr");
    const name = document.createElement("td");
    const description = document.createElement("td");
    name.textContent = tag.name;
    description.textContent = tag.description;
    row.append(name, description);
    return row;
  }));
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  error.hidden = true;
  try {
    await api("/api/tags", {
      method: "POST",
      body: {
        name: form.elements.name.value,
        description: form.elements.description.value,
      },
    });
    form.reset();
    await load();
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  }
});

try {
  await load();
} catch (err) {
  error.textContent = err.message;
  error.hidden = false;
}
