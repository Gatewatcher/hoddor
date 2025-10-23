import { Tabs } from 'antd';
import { useState } from 'react';
import { Provider } from 'react-redux';

import { RAGWorkspace } from './components/RAGWorkspace';
import { Vaults } from './components/Vaults';
import { ServicesProvider } from './contexts/ServicesContext';
import { reduxStore } from './store/app.store';

export const App = () => {
  const [activeTab, setActiveTab] = useState('vaults');

  window.addEventListener('message', event => {
    if (event.data.event === 'vaultUpdate') {
      console.log('Received update from Hoddor using window object!');
      console.log('event.data', event.data);
    }
  });

  return (
    <Provider store={reduxStore}>
      <ServicesProvider>
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          centered
          items={[
            {
              key: 'vaults',
              label: 'Vaults',
              children: <Vaults />,
            },
            {
              key: 'rag',
              label: 'RAG + Graph',
              children: <RAGWorkspace />,
            },
          ]}
        />
      </ServicesProvider>
    </Provider>
  );
};
