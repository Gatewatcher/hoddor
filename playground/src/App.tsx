import { useState } from 'react';
import { Provider } from 'react-redux';
import { Tabs } from 'antd';

import { Vaults } from './components/Vaults';
import { LLMChat } from './components/LLMChat';
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
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: 'vaults',
            label: 'Vaults',
            children: <Vaults />,
          },
          {
            key: 'llm',
            label: 'LLM Chat',
            children: <LLMChat />,
          },
        ]}
        style={{ height: '100vh', padding: '16px' }}
      />
    </Provider>
  );
};
