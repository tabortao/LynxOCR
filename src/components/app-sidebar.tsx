import { useState, useEffect, type ComponentProps } from "react"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
} from "@/components/ui/sidebar"
import {
  ScanTextIcon,
  Settings2Icon,
  CpuIcon,
  NetworkIcon,
  InfoIcon,
  CommandIcon,
  SunIcon,
  MoonIcon,
  LanguagesIcon,
} from "lucide-react"
import { getVersion } from "@tauri-apps/api/app"
import { useAppContext } from "@/lib/app-context"
import type { Page } from "@/App"

interface AppSidebarProps extends ComponentProps<typeof Sidebar> {
  currentPage: Page
  onNavigate: (page: Page) => void
}

export function AppSidebar({
  currentPage,
  onNavigate,
  ...props
}: AppSidebarProps) {
  const { theme, toggleTheme, language, setLanguage, t } = useAppContext()
  const [appVersion, setAppVersion] = useState("")

  useEffect(() => {
    getVersion().then((v) => setAppVersion(v)).catch(() => setAppVersion(""))
  }, [])

  const mainNav = [
    { id: "ocr" as Page, title: t("ocr"), icon: ScanTextIcon },
  ]

  const settingsNav = [
    { id: "settings" as Page, title: t("settings"), icon: Settings2Icon },
    { id: "model-settings" as Page, title: t("models"), icon: CpuIcon },
    { id: "api-settings" as Page, title: t("api"), icon: NetworkIcon },
  ]

  return (
    <Sidebar collapsible="offcanvas" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              className="data-[slot=sidebar-menu-button]:p-1.5!"
            >
              <a href="#" onClick={(e) => { e.preventDefault(); onNavigate("ocr") }}>
                <CommandIcon className="size-5!" />
                <span className="text-base font-semibold">LynxOCR</span>
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>{t("features")}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {mainNav.map((item) => (
                <SidebarMenuItem key={item.id}>
                  <SidebarMenuButton
                    tooltip={item.title}
                    isActive={currentPage === item.id}
                    onClick={() => onNavigate(item.id)}
                  >
                    <item.icon />
                    <span>{item.title}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup className="mt-auto">
          <SidebarGroupLabel>{t("settingsLabel")}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {settingsNav.map((item) => (
                <SidebarMenuItem key={item.id}>
                  <SidebarMenuButton
                    tooltip={item.title}
                    isActive={currentPage === item.id}
                    onClick={() => onNavigate(item.id)}
                  >
                    <item.icon />
                    <span>{item.title}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
              <SidebarMenuItem>
                <SidebarMenuButton
                  tooltip={t("about")}
                  isActive={currentPage === "about"}
                  onClick={() => onNavigate("about")}
                >
                  <InfoIcon />
                  <span>{t("about")}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <div className="flex items-center justify-between px-3 py-2">
          <span className="text-xs text-muted-foreground">{appVersion ? `v${appVersion}` : ""}</span>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setLanguage(language === "zh" ? "en" : "zh")}
              className="p-1 rounded hover:bg-accent transition-colors"
              title={language === "zh" ? "Switch to English" : "切换到中文"}
            >
              <LanguagesIcon className="size-3.5 text-muted-foreground" />
            </button>
            <button
              onClick={toggleTheme}
              className="p-1 rounded hover:bg-accent transition-colors"
              title={theme === "dark" ? "Switch to light mode" : "切换到深色模式"}
            >
              {theme === "dark" ? (
                <SunIcon className="size-3.5 text-muted-foreground" />
              ) : (
                <MoonIcon className="size-3.5 text-muted-foreground" />
              )}
            </button>
          </div>
        </div>
      </SidebarFooter>
    </Sidebar>
  )
}