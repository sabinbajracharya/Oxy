import './styles/global.css';
import './styles/home.css';
import './styles/playground.css';
import './styles/tour.css';
import './styles/components.css';

import { Router } from './router';
import { Header } from './components/Header';
import { Footer } from './components/Footer';

const app = document.getElementById('app')!;
const header = new Header();
const footer = new Footer();

header.render(app);

const viewContainer = document.createElement('main');
viewContainer.id = 'view';
app.appendChild(viewContainer);

footer.render(app);

const router = new Router(viewContainer);

// Lazy imports to code-split heavy deps
router
  .add(/^$/, async () => new (await import('./views/HomeView')).HomeView())
  .add(/^playground$/, async () => new (await import('./views/PlaygroundView')).PlaygroundView())
  .add(/^tour\/contents$/, async () => new (await import('./views/TourContentsView')).TourContentsView())
  .add(/^tour\/([^/]+)\/([^/]+)$/, async (m) => {
    const [, chapter, lesson] = m;
    return new (await import('./views/TourView')).TourView(chapter, lesson);
  });

router.start();
