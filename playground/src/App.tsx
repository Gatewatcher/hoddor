import { Provider } from 'react-redux';

import { Vaults } from './components/Vaults';
import { reduxStore } from './store/app.store';

export const App = () => {
  window.addEventListener('message', event => {
    if (event.data.event === 'vaultUpdate') {
      console.log('Received update from Hoddor using window object!');
      console.log('event.data', event.data);
    }
  });
  return (
    <Provider store={reduxStore}>
      <Vaults />
    </Provider>
  );
};
