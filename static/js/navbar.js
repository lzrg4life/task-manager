import { api, logout } from "./api.js";

const NAV_ITEMS = [
  { href: "/tickets.html", label: "Tickets", paths: ["/tickets.html", "/ticket.html"] },
  { href: "/archived.html", label: "Archived", paths: ["/archived.html"] },
  { href: "/removed.html", label: "Removed", paths: ["/removed.html"] },
];

let navbarPromise;

function createLink({ href, label, paths }) {
  const link = document.createElement("a");
  link.href = href;
  link.textContent = label;
  if (paths.includes(location.pathname)) link.setAttribute("aria-current", "page");
  return link;
}

function createNavbar(me) {
  const header = document.createElement("header");
  header.className = "topbar";

  const navigation = document.createElement("nav");
  navigation.setAttribute("aria-label", "Main navigation");
  const appName = document.createElement("strong");
  appName.textContent = "Task Manager";
  navigation.append(appName);

  for (const item of NAV_ITEMS) navigation.append(" · ", createLink(item));
  if (me.is_admin) {
    navigation.append(" · ", createLink({
      href: "/users.html",
      label: "Users",
      paths: ["/users.html"],
    }));
    navigation.append(" · ", createLink({
      href: "/tags.html",
      label: "Tags",
      paths: ["/tags.html"],
    }));
  }

  const account = document.createElement("span");
  account.append(`${me.email} `);
  const signOut = document.createElement("button");
  signOut.type = "button";
  signOut.className = "secondary";
  signOut.textContent = "Sign out";
  signOut.addEventListener("click", logout);
  account.append(signOut);

  header.append(navigation, account);
  document.body.prepend(header);
}

export function initNavbar() {
  navbarPromise ??= api("/api/me").then((me) => {
    if (!me) throw new Error("not signed in");
    createNavbar(me);
    return me;
  });
  return navbarPromise;
}
