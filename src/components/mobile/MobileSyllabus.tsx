import React, { useState, useRef } from 'react';
import {
  View,
  Text,
  TouchableOpacity,
  ScrollView,
  StyleSheet,
  Animated,
  LayoutAnimation,
  UIManager,
  Platform,
} from 'react-native';
import { Section, Lesson, CourseProgress } from '../../types/course';

// Enable LayoutAnimation on Android
if (Platform.OS === 'android' && UIManager.setLayoutAnimationEnabledExperimental) {
  UIManager.setLayoutAnimationEnabledExperimental(true);
}

interface MobileSyllabusProps {
  sections: Section[];
  progress?: CourseProgress | null;
  currentLessonId?: string;
  onLessonSelect: (lessonId: string, sectionId: string) => void;
  onSectionToggle?: (sectionId: string, isExpanded: boolean) => void;
}

export default function MobileSyllabus({
  sections,
  progress,
  currentLessonId,
  onLessonSelect,
  onSectionToggle,
}: MobileSyllabusProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(sections.map((s) => s.id)) // All expanded by default
  );

  const toggleSection = (sectionId: string) => {
    const newExpanded = new Set(expandedSections);
    const isCurrentlyExpanded = newExpanded.has(sectionId);

    if (isCurrentlyExpanded) {
      newExpanded.delete(sectionId);
    } else {
      newExpanded.add(sectionId);
    }

    setExpandedSections(newExpanded);
    onSectionToggle?.(sectionId, !isCurrentlyExpanded);
  };

  const getSectionProgress = (section: Section): number => {
    if (!progress || section.lessons.length === 0) return 0;

    const completedCount = section.lessons.filter(
      (lesson) => progress.lessons[lesson.id]?.completed
    ).length;

    return Math.round((completedCount / section.lessons.length) * 100);
  };

  const getLessonStatus = (lesson: Lesson): 'completed' | 'in-progress' | 'not-started' => {
    if (!progress) return 'not-started';
    
    const lessonProgress = progress.lessons[lesson.id];
    if (lessonProgress?.completed) return 'completed';
    if (lesson.id === currentLessonId || lessonProgress?.lastPosition > 0) {
      return 'in-progress';
    }
    return 'not-started';
  };

  return (
    <ScrollView
      className="flex-1 bg-background-light dark:bg-slate-900"
      contentContainerStyle={styles.container}
      showsVerticalScrollIndicator={true}
    >
      <View className="px-4 py-4 bg-white dark:bg-slate-800 border-b border-gray-200 dark:border-slate-700 shadow-sm">
        <Text className="text-2xl font-bold text-gray-900 dark:text-white">
          📚 Course Syllabus
        </Text>
        <Text className="text-sm text-gray-600 dark:text-gray-400 mt-2 font-medium">
          {sections.length} sections • {sections.reduce((acc, s) => acc + s.lessons.length, 0)} lessons
        </Text>
      </View>

      {sections.map((section) => {
        const isExpanded = expandedSections.has(section.id);
        const sectionProgress = getSectionProgress(section);

        return (
          <View
            key={section.id}
            className="mb-3 mx-3 bg-white dark:bg-slate-800 rounded-xl border border-gray-200 dark:border-slate-700 overflow-hidden shadow-sm mt-3"
          >
            {/* Section Header */}
            <TouchableOpacity
              onPress={() => toggleSection(section.id)}
              className="flex-row items-center justify-between px-4 py-4 bg-gradient-to-r from-gradient-start/5 to-gradient-end/5 dark:from-slate-700 dark:to-slate-800 active:bg-gradient-start/10"
            >
              <View className="flex-1 mr-3">
                <View className="flex-row items-center mb-2">
                  <Text className="text-lg font-bold text-gray-900 dark:text-white flex-1">
                    {section.title}
                  </Text>
                  <View className="px-2.5 py-1 bg-primary/15 rounded-full ml-2">
                    <Text className="text-xs font-semibold text-primary">
                      {section.lessons.length}
                    </Text>
                  </View>
                </View>
                
                {/* Progress Bar */}
                <View className="h-1.5 bg-gray-200 dark:bg-slate-700 rounded-full mt-2 overflow-hidden">
                  <View
                    className="h-full bg-gradient-to-r from-gradient-start via-gradient-mid to-gradient-end"
                    style={{ width: `${sectionProgress}%` }}
                  />
                </View>
                <Text className="text-xs font-semibold text-primary mt-1.5">
                  {sectionProgress}% complete
                </Text>
              </View>

              {/* Expand/Collapse Icon */}
              <View>
                <Text 
                  className="text-2xl text-primary"
                  style={{
                    transform: [{ rotate: isExpanded ? '180deg' : '0deg' }],
                  }}
                >
                  ▼
                </Text>
              </View>
            </TouchableOpacity>

            {/* Section Lessons */}
            {isExpanded && (
              <View className="border-t border-gray-200 dark:border-slate-700">
                {section.lessons.map((lesson, lessonIndex) => {
                  const status = getLessonStatus(lesson);
                  const isCurrent = lesson.id === currentLessonId;
                  const lessonProgress = progress?.lessons[lesson.id];

                  return (
                    <TouchableOpacity
                      key={lesson.id}
                      onPress={() => onLessonSelect(lesson.id, section.id)}
                      className={`px-4 py-3 border-l-4 flex-row items-start ${
                        isCurrent
                          ? 'bg-primary/10 dark:bg-primary/5 border-primary'
                          : 'bg-white dark:bg-slate-800 border-transparent'
                      } active:bg-gray-50 dark:active:bg-slate-700`}
                    >
                      {/* Lesson Status Icon */}
                      <View className="mr-3 mt-0.5">
                        {status === 'completed' ? (
                          <View className="w-6 h-6 rounded-full bg-green-500 items-center justify-center shadow-sm">
                            <Text className="text-white text-sm font-bold">✓</Text>
                          </View>
                        ) : status === 'in-progress' ? (
                          <View className="w-6 h-6 rounded-full bg-primary items-center justify-center shadow-sm">
                            <View className="w-2 h-2 rounded-full bg-white" />
                          </View>
                        ) : (
                          <View className="w-6 h-6 rounded-full border-2 border-gray-300 dark:border-slate-600 items-center justify-center bg-gray-50 dark:bg-slate-700">
                            <Text className="text-xs font-bold text-gray-500 dark:text-gray-400">
                              {lessonIndex + 1}
                            </Text>
                          </View>
                        )}
                      </View>

                      {/* Lesson Info */}
                      <View className="flex-1">
                        <Text
                          className={`text-base font-semibold mb-1 ${
                            isCurrent
                              ? 'text-primary'
                              : 'text-gray-900 dark:text-white'
                          }`}
                        >
                          {lesson.title}
                        </Text>
                        
                        <View className="flex-row items-center flex-wrap gap-2">
                          <View className="flex-row items-center bg-gray-100 dark:bg-slate-700 px-2 py-1 rounded">
                            <Text className="text-xs font-medium text-gray-700 dark:text-gray-300">
                              ⏱️ {lesson.duration} min
                            </Text>
                          </View>
                          
                          {lessonProgress?.lastPosition > 0 && !status === 'completed' && (
                            <View className="flex-row items-center bg-primary/10 px-2 py-1 rounded">
                              <Text className="text-xs font-semibold text-primary">
                                📌 Resume
                              </Text>
                            </View>
                          )}
                          
                          {progress?.bookmarks.includes(lesson.id) && (
                            <View className="flex-row items-center bg-yellow-50 dark:bg-yellow-900/20 px-2 py-1 rounded">
                              <Text className="text-xs font-semibold text-yellow-600 dark:text-yellow-400">
                                ⭐ Bookmarked
                              </Text>
                            </View>
                          )}
                        </View>
                      </View>

                      {/* Current Lesson Badge */}
                      {isCurrent && (
                        <View className="ml-2 px-2.5 py-1 bg-primary rounded-full">
                          <Text className="text-xs font-bold text-white">
                            Current
                          </Text>
                        </View>
                      )}
                    </TouchableOpacity>
                  );
                })}
              </View>
            )}
          </View>
        );
      })}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    paddingBottom: 32,
  },
});
