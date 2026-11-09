// Full desktop application shell (docs/design/SCREENS.md §3.1).
// Native-feeling sidebar with roving-tabindex nav, skip link, route h1 focus,
// document.title updates and the five workspace routes.

import { useEffect, useRef, useState } from "react";
import { useApp } from "../App";
import { Logo } from "../components/Logo";
import HomeView from "./views/HomeView";
import IndexesView from "./views/IndexesView";
import ActivityView from "./views/ActivityView";
import WorkflowsView from "./views/WorkflowsView";
import PreferencesView from "./views/PreferencesView";
import { SECTION_TITLES, type Section } from "./sections";
import { takeOnboardingCrossfade } from "./transition";
import { Icon, type IconName } from "./ui";
import "./desktop.css";

interface NavItem {
  id: Section;
  label: string;
  icon: IconName;
}

const NAV: NavItem[] = [
  { id: "home", label: "Home", icon: "home" },
  { id: "indexes", label: "Search & indexes", icon: "search" },
  { id: "activity", label: "Activity", icon: "activity" },
  { id: "workflows", label: "Workflows", icon: "workflows" },
  { id: "preferences", label: "Preferences", icon: "preferences" },
];

export default function DesktopApp() {
  const { settings } = useApp();
  const [section, setSection] = useState<Section>("home");
  const [pendingSource, setPendingSource] = useState<string | null>(null);
  const [navFocus, setNavFocus] = useState(0);
  // 160 ms fade/raise when the main window is shown (app start, tray "Open
  // ProtonSearch", launcher footer). Route changes never replay it.
  const [opening, setOpening] = useState(() => !takeOnboardingCrossfade());
  const navRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const mainRef = useRef<HTMLElement>(null);

  const userName = settings.userName?.trim() || null;
  const initial = userName ? userName.charAt(0).toUpperCase() : null;

  useEffect(() => {
    if (!opening) return;
    // animationend normally clears this; the timer is the reduced-motion /
    // no-animation-environment fallback so the class never sticks.
    const timer = window.setTimeout(() => setOpening(false), 240);
    return () => window.clearTimeout(timer);
  }, [opening]);

  useEffect(() => {
    let hidden = document.visibilityState === "hidden";
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        if (hidden) setOpening(true);
        hidden = false;
      } else {
        hidden = true;
      }
    };
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => document.removeEventListener("visibilitychange", onVisibilityChange);
  }, []);

  useEffect(() => {
    document.title = SECTION_TITLES[section] + " — ProtonSearch";
    const activeIndex = NAV.findIndex((item) => item.id === section);
    if (activeIndex >= 0) setNavFocus(activeIndex);
  }, [section]);

  const navigate = (next: Section, sourceId?: string) => {
    setPendingSource(sourceId ?? null);
    setSection(next);
  };

  const onNavKeyDown = (event: React.KeyboardEvent<HTMLElement>) => {
    let next: number | null = null;
    if (event.key === "ArrowDown") next = (navFocus + 1) % NAV.length;
    else if (event.key === "ArrowUp") next = (navFocus - 1 + NAV.length) % NAV.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = NAV.length - 1;
    if (next === null) return;
    event.preventDefault();
    setNavFocus(next);
    navRefs.current[next]?.focus();
  };

  const skipToMain = () => {
    mainRef.current?.focus();
  };

  return (
    <div
      className={"app-shell" + (opening ? " is-opening" : "")}
      onAnimationEnd={(event) => {
        if (event.target === event.currentTarget) setOpening(false);
      }}
    >
      <a
        className="skip-link"
        href="#main"
        onClick={(event) => {
          event.preventDefault();
          skipToMain();
        }}
      >
        Skip to main content
      </a>

      <aside className="sidebar">
        <div className="sidebar-brand">
          <span aria-hidden="true">
            <Logo size={29} />
          </span>
          <span className="brand-name">ProtonSearch</span>
        </div>
        <nav aria-label="Main">
          <p className="sidebar-label" id="nav-label">
            Workspace
          </p>
          <ul className="nav-list" aria-labelledby="nav-label">
            {NAV.map((item, index) => (
              <li key={item.id}>
                <button
                  ref={(element) => {
                    navRefs.current[index] = element;
                  }}
                  type="button"
                  className="navbtn"
                  aria-current={section === item.id ? "page" : undefined}
                  tabIndex={navFocus === index ? 0 : -1}
                  title={item.label}
                  onClick={() => navigate(item.id)}
                  onFocus={() => setNavFocus(index)}
                  onKeyDown={onNavKeyDown}
                >
                  <span className="navglyph" aria-hidden="true">
                    <Icon name={item.icon} size={16} />
                  </span>
                  <span className="navtext">{item.label}</span>
                </button>
              </li>
            ))}
          </ul>
        </nav>
        <div className="sidebar-foot">
          <div className="userfoot">
            <span className="avatar" aria-hidden="true">
              {initial ?? <Icon name="person" size={16} />}
            </span>
            <span className="user-name">{userName ?? "This PC"}</span>
          </div>
        </div>
      </aside>

      <main id="main" className="app-main" tabIndex={-1} ref={mainRef}>
        <div className="workspace">
          {section === "home" ? <HomeView onNavigate={navigate} /> : null}
          {section === "indexes" ? <IndexesView focusSource={pendingSource} /> : null}
          {section === "activity" ? <ActivityView /> : null}
          {section === "workflows" ? <WorkflowsView /> : null}
          {section === "preferences" ? <PreferencesView onNavigate={navigate} /> : null}
        </div>
      </main>

      <div className="sr-only" role="status" aria-live="polite">
        {SECTION_TITLES[section]}
      </div>
    </div>
  );
}