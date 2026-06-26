import { ref, computed } from 'vue';
import {
  type RightPanelType,
  DEFAULT_PANEL_WIDTHS,
  loadPanelWidths,
  savePanelWidths as persistPanelWidths,
  getPanelMinWidth,
} from '../utils/rightPanel.ts';

export function useRightPanel() {
  const rightPanelVisible = ref(false);
  const rightPanelType = ref<RightPanelType>('quick');
  const panelWidths = ref<Record<RightPanelType, number>>(loadPanelWidths());
  const isResizing = ref(false);

  const panelWidth = computed(() =>
    panelWidths.value[rightPanelType.value] ?? DEFAULT_PANEL_WIDTHS[rightPanelType.value],
  );

  const savePanelWidths = () => {
    persistPanelWidths(panelWidths.value);
  };

  const reloadPanelWidths = () => {
    panelWidths.value = loadPanelWidths();
  };

  const startResizing = (e: MouseEvent) => {
    isResizing.value = true;
    const startX = e.clientX;
    const panelType = rightPanelType.value;
    const startWidth = panelWidths.value[panelType];

    const doResize = (moveEvent: MouseEvent) => {
      if (!isResizing.value) return;
      const delta = moveEvent.clientX - startX;
      const newWidth = startWidth - delta;
      const maxWidth = window.innerWidth - 300;
      const minWidth = getPanelMinWidth(panelType);
      if (newWidth >= minWidth && newWidth <= maxWidth) {
        panelWidths.value = { ...panelWidths.value, [panelType]: newWidth };
      }
    };

    const stopResizing = () => {
      isResizing.value = false;
      document.removeEventListener('mousemove', doResize);
      document.removeEventListener('mouseup', stopResizing);
      document.body.style.cursor = 'default';
      savePanelWidths();
    };

    document.addEventListener('mousemove', doResize);
    document.addEventListener('mouseup', stopResizing);
    document.body.style.cursor = 'col-resize';
  };

  return {
    rightPanelVisible,
    rightPanelType,
    panelWidths,
    panelWidth,
    isResizing,
    savePanelWidths,
    reloadPanelWidths,
    startResizing,
    getPanelMinWidth,
  };
}

export type { RightPanelType };
