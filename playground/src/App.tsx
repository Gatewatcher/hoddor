import { Provider } from 'react-redux';

import { Vaults } from './components/Vaults';
import { reduxStore } from './store/app.store';

export const App = () => {
  return (
    <Provider store={reduxStore}>
      <Vaults />
    </Provider>
  );
};
