import { FocusFlowWidget } from './components/FocusFlowWidget/FocusFlowWidget';
import styles from './App.module.css';

export default function App() {
  return (
    <main className={styles.stage}>
      <div className={styles.previewShell}>
        <FocusFlowWidget />
      </div>
      <p className={styles.previewHint}>* 桌面浮窗应用概念预览</p>
    </main>
  );
}
