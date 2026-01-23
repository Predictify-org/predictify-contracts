import React from 'react';
import {
  TouchableOpacity,
  Text,
  ActivityIndicator,
  View,
  ViewStyle,
  TextStyle,
  StyleSheet,
} from 'react-native';
import { LinearGradient } from 'expo-linear-gradient';

interface PrimaryButtonProps {
  onPress: () => void;
  title: string;
  loading?: boolean;
  disabled?: boolean;
  variant?: 'gradient' | 'solid' | 'outline';
  size?: 'small' | 'medium' | 'large';
  style?: ViewStyle;
  textStyle?: TextStyle;
  icon?: React.ReactNode;
}

export default function PrimaryButton({
  onPress,
  title,
  loading = false,
  disabled = false,
  variant = 'gradient',
  size = 'medium',
  style,
  textStyle,
  icon,
}: PrimaryButtonProps) {
  const isDisabled = loading || disabled;

  const sizeConfig = {
    small: { paddingHorizontal: 12, paddingVertical: 8, borderRadius: 8, fontSize: 14 },
    medium: { paddingHorizontal: 24, paddingVertical: 12, borderRadius: 12, fontSize: 16 },
    large: { paddingHorizontal: 32, paddingVertical: 16, borderRadius: 12, fontSize: 18 },
  };

  const config = sizeConfig[size];

  if (variant === 'gradient') {
    return (
      <TouchableOpacity
        onPress={onPress}
        disabled={isDisabled}
        activeOpacity={0.8}
        style={[{ opacity: isDisabled ? 0.6 : 1 }, style]}
      >
        <LinearGradient
          colors={['#20afe7', '#2c8aec', '#586ce9']}
          start={{ x: 0, y: 0 }}
          end={{ x: 1, y: 0 }}
          style={[
            styles.button,
            styles.gradientButton,
            {
              paddingHorizontal: config.paddingHorizontal,
              paddingVertical: config.paddingVertical,
              borderRadius: config.borderRadius,
            },
          ]}
        >
          {loading ? (
            <ActivityIndicator color="white" size="small" />
          ) : (
            <>
              {icon}
              <Text
                style={[
                  styles.buttonText,
                  { fontSize: config.fontSize, color: '#ffffff' },
                  textStyle,
                ]}
              >
                {title}
              </Text>
            </>
          )}
        </LinearGradient>
      </TouchableOpacity>
    );
  }

  if (variant === 'solid') {
    return (
      <TouchableOpacity
        onPress={onPress}
        disabled={isDisabled}
        activeOpacity={0.8}
        style={[
          styles.button,
          {
            backgroundColor: '#19c3e6',
            paddingHorizontal: config.paddingHorizontal,
            paddingVertical: config.paddingVertical,
            borderRadius: config.borderRadius,
            opacity: isDisabled ? 0.6 : 1,
          },
          style,
        ]}
      >
        {loading ? (
          <ActivityIndicator color="white" size="small" />
        ) : (
          <>
            {icon}
            <Text
              style={[
                styles.buttonText,
                { fontSize: config.fontSize, color: '#ffffff' },
                textStyle,
              ]}
            >
              {title}
            </Text>
          </>
        )}
      </TouchableOpacity>
    );
  }

  // Outline variant
  return (
    <TouchableOpacity
      onPress={onPress}
      disabled={isDisabled}
      activeOpacity={0.8}
      style={[
        styles.button,
        {
          borderWidth: 2,
          borderColor: '#19c3e6',
          backgroundColor: '#f0f1f5',
          paddingHorizontal: config.paddingHorizontal,
          paddingVertical: config.paddingVertical,
          borderRadius: config.borderRadius,
          opacity: isDisabled ? 0.6 : 1,
        },
        style,
      ]}
    >
      {loading ? (
        <ActivityIndicator color="#19c3e6" size="small" />
      ) : (
        <>
          {icon}
          <Text
            style={[
              styles.buttonText,
              { fontSize: config.fontSize, color: '#19c3e6' },
              textStyle,
            ]}
          >
            {title}
          </Text>
        </>
      )}
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  button: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
  },
  gradientButton: {
    shadowColor: '#20afe7',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 4,
  },
  buttonText: {
    fontWeight: '600',
  },
});
