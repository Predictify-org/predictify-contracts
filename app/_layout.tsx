import React from 'react';
import { View } from 'react-native';
import { SwipeableNavigation } from '../src/components/mobile/SwipeableNavigation';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import 'react-native-reanimated';
import "../global.css"; // NativeWind CSS

export default function RootLayout() {
  return (
    <GestureHandlerRootView style={{ flex: 1 }}>
      <SwipeableNavigation />
    </GestureHandlerRootView>
  );
}
