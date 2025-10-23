import { ClearOutlined, SendOutlined } from '@ant-design/icons';
import { Button, Input, Space } from 'antd';
import React from 'react';

const { TextArea } = Input;

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onClear: () => void;
  disabled?: boolean;
  loading?: boolean;
}

export const ChatInput = ({
  value,
  onChange,
  onSend,
  onClear,
  disabled = false,
  loading = false,
}: ChatInputProps) => {
  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      onSend();
    }
  };

  return (
    <Space.Compact style={{ width: '100%' }}>
      <TextArea
        value={value}
        onChange={e => onChange(e.target.value)}
        onKeyPress={handleKeyPress}
        placeholder="Type your message... (Enter to send, Shift+Enter for new line)"
        autoSize={{ minRows: 1, maxRows: 4 }}
        disabled={disabled || loading}
      />
      <Button
        type="primary"
        icon={<SendOutlined />}
        onClick={onSend}
        loading={loading}
        disabled={disabled || !value.trim()}
      >
        Send
      </Button>
      <Button icon={<ClearOutlined />} onClick={onClear} disabled={disabled}>
        Clear
      </Button>
    </Space.Compact>
  );
};
