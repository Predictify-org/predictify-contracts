import React from 'react';
import { NativeStackScreenProps } from '@react-navigation/native-stack';
import { RootStackParamList } from '../navigation/types';
import MobileCourseViewer from '../components/mobile/MobileCourseViewer';

type Props = NativeStackScreenProps<RootStackParamList, 'CourseViewer'>;

export default function CourseViewerScreen({ route, navigation }: Props) {
  const { course, initialLessonId } = route.params;

  return (
    <MobileCourseViewer
      course={course}
      initialLessonId={initialLessonId}
      onBack={() => navigation.goBack()}
    />
  );
}
