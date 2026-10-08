import { api, logout } from "./api.js";

const status = document.getElementById("status");
const me = await api("/api/me");
if (me) {
  status.textContent = `Signed in as ${me.email} (${me.roles.join(", ")}). `;
  if (me.roles.includes("admin")) {
    const link = document.createElement("a");
    link.href = "/users.html";
    link.textContent = "Manage users";
    status.append(link, " ");
  }
  const btn = document.createElement("button");
  btn.textContent = "Sign out";
  btn.className = "secondary";
  btn.addEventListener("click", logout);
  status.append(btn);
}
