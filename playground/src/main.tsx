import ReactDOM from 'react-dom/client';
import init, { set_debug_mode } from "../../hoddor/pkg/hoddor";

import {App} from './App';

async function bootstrap() {
  await init();
  set_debug_mode(true);
    
  const container = document.getElementById('app');
  const app = ReactDOM.createRoot(container as Element);
  app.render(<App />);
}

bootstrap();
