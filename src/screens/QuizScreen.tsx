import React from 'react';
import { NativeStackScreenProps } from '@react-navigation/native-stack';
import { RootStackParamList } from '../navigation/types';
import MobileQuizManager from '../components/mobile/MobileQuizManager';

type Props = NativeStackScreenProps<RootStackParamList, 'Quiz'>;

export default function QuizScreen({ route, navigation }: Props) {
  const { quiz, courseId, course } = route.params;

  return (
    <MobileQuizManager
      quiz={quiz}
      courseId={courseId}
      course={course}
      navigation={navigation}
      onBack={() => navigation.goBack()}
    />
  );
}
