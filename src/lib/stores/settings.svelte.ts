import { getSettings, updateSettings } from '../utils/tauri';
import type { AppConfig } from '../types';

class SettingsStore {
  config = $state<AppConfig | null>(null);

  async loadSettings() {
    try {
      const settings = await getSettings();
      this.config = settings;
    } catch (err) {
      console.error('Failed to load settings:', err);
    }
  }

  async saveSettings(newConfig: AppConfig) {
    try {
      await updateSettings(newConfig);
      // Deep copy to trigger reactivity
      this.config = JSON.parse(JSON.stringify(newConfig));
    } catch (err) {
      console.error('Failed to save settings:', err);
      throw err;
    }
  }
}

export const settingsStore = new SettingsStore();
