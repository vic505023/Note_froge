type ViewMode = 'edit' | 'preview';
type SidebarView = 'notebooks' | 'files';
type SidebarTab = 'notes' | 'sources';

class UIStore {
  sidebarOpen = $state(true);
  aiPanelOpen = $state(true);
  viewMode = $state<ViewMode>('edit');
  sidebarView = $state<SidebarView>('notebooks');
  sidebarTab = $state<SidebarTab>('notes');

  toggleSidebar() {
    this.sidebarOpen = !this.sidebarOpen;
  }

  toggleAIPanel() {
    this.aiPanelOpen = !this.aiPanelOpen;
  }

  cycleViewMode() {
    this.viewMode = this.viewMode === 'edit' ? 'preview' : 'edit';
  }

  setViewMode(mode: ViewMode) {
    this.viewMode = mode;
  }

  setSidebarView(view: SidebarView) {
    this.sidebarView = view;
  }

  setSidebarTab(tab: SidebarTab) {
    this.sidebarTab = tab;
  }
}

export const uiStore = new UIStore();
