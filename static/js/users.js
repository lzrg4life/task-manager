import { api, logout } from "./api.js";
import { showAdminUsersLink } from "./ticket-common.js";

const rows = document.getElementById("user-rows");
const pageError = document.getElementById("error");
const dialog = document.getElementById("user-dialog");
const form = document.getElementById("user-form");
const dialogError = document.getElementById("dialog-error");
let editingId = null;

function show(el, msg) { el.textContent = msg; el.hidden = !msg; }

const me = await api("/api/me");
if (!me) throw new Error("not signed in");
if (!me.is_admin) { location.href = "/"; throw new Error("not an admin"); }
document.getElementById("whoami").textContent = me.email;
document.getElementById("logout").addEventListener("click", logout);
showAdminUsersLink(me);

function cell(text) {
  const td = document.createElement("td");
  td.textContent = text;
  return td;
}

async function load() {
  try {
    const users = await api("/api/users");
    rows.replaceChildren(...users.map((u) => {
      const tr = document.createElement("tr");
      tr.append(cell(u.email), cell(u.display_name));
      const edit = document.createElement("button");
      edit.textContent = "Edit";
      edit.className = "secondary";
      edit.addEventListener("click", () => openDialog(u));
      const actions = document.createElement("td");
      actions.append(edit);
      tr.append(cell(u.is_admin ? "Yes" : "No"), cell(u.is_active ? "Yes" : "No"), actions);
      return tr;
    }));
    show(pageError, "");
  } catch (err) { show(pageError, err.message); }
}

function openDialog(user) {
  editingId = user ? user.id : null;
  form.reset();
  show(dialogError, "");
  document.getElementById("dialog-title").textContent = user ? "Edit user" : "Add user";
  document.getElementById("password-label").textContent = user ? "New password (leave blank to keep)" : "Password";
  form.elements.password.required = !user;
  form.elements.email.value = user ? user.email : "";
  form.elements.display_name.value = user ? user.display_name : "";
  form.elements.is_active.checked = user ? user.is_active : true;
  document.getElementById("active-row").hidden = !user;
  form.elements.is_admin.checked = user ? user.is_admin : false;
  dialog.showModal();
}

document.getElementById("add-user").addEventListener("click", () => openDialog(null));
document.getElementById("cancel").addEventListener("click", () => dialog.close());
form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const body = {
    email: form.elements.email.value,
    display_name: form.elements.display_name.value,
    is_admin: form.elements.is_admin.checked,
  };
  if (form.elements.password.value) body.password = form.elements.password.value;
  try {
    if (editingId === null) {
      await api("/api/users", { method: "POST", body });
    } else {
      body.is_active = form.elements.is_active.checked;
      await api(`/api/users/${editingId}`, { method: "PATCH", body });
    }
    dialog.close();
    await load();
  } catch (err) { show(dialogError, err.message); }
});

await load();
