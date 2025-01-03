import { z } from 'zod'

export const UserIdSchema = z.string()
export const TeamIdSchema = z.string()
export const TaskIdSchema = z.string()
export const TestIdSchema = z.string()

export const AbortedExecutionSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
  runtime: z.number().describe('duration in ms'),
})

export const ExecutionExitStatusSchema = z.union([
  z.literal('Aborted'),
  z.literal('Error'),
  z.literal('Finished'),
  z.literal('Timeout'),
])

// Out of order due to dependencies
export const InternalErrorSchema = z.object({
  message: z.string(),
  runtime: z.number().describe('duration in ms'),
})

// Out of order due to dependencies
export const FinishedExecutionSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
  runtime: z.number().describe('duration in ms'),
  exitStatus: z.number().nullable(),
})

export const ExecutionOutputSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('Aborted') }).merge(AbortedExecutionSchema),
  z.object({ type: z.literal('Error') }).merge(InternalErrorSchema),
  z.object({ type: z.literal('Finished') }).merge(FinishedExecutionSchema),
  z.object({ type: z.literal('Timeout') }).merge(FinishedExecutionSchema),
])

// Out of order due to dependencies
export const FinishedTaskInfoSchema = z.object({
  taskId: TaskIdSchema,
  start: z.number().transform((ms) => new Date(ms)),
  end: z.number().transform((ms) => new Date(ms)),
  teamId: TeamIdSchema,
  revisionId: z.string(),
})

// Out of order due to dependencies
export const FinishedTestSchema = z.object({
  testId: z.string(),
  output: ExecutionOutputSchema,
})

export const FinishedCompilerTaskSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BuildFailed'),
    info: FinishedTaskInfoSchema,
    buildOutput: ExecutionOutputSchema,
  }),
  z.object({
    type: z.literal('RanTests'),
    info: FinishedTaskInfoSchema,
    buildOutput: FinishedExecutionSchema,
    tests: z.array(FinishedTestSchema),
  }),
])

// Out of order due to dependencies
export const FinishedTestSummarySchema = z.object({
  testId: TestIdSchema,
  output: ExecutionExitStatusSchema,
})

export const FinishedCompilerTaskSummarySchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BuildFailed'),
    info: FinishedTaskInfoSchema,
    status: ExecutionExitStatusSchema,
  }),
  z.object({
    type: z.literal('RanTests'),
    info: FinishedTaskInfoSchema,
    tests: z.array(FinishedTestSummarySchema),
  }),
])

export const RepoSchema = z.object({
  team: TeamIdSchema,
  url: z.string().url('The repo URL is invalid'),
  autoFetch: z.boolean(),
})

// Out of order due to dependencies
export const UserSchema = z.object({
  id: UserIdSchema,
  displayName: z.string(),
  team: TeamIdSchema.nullable(),
})

// Out of order due to dependencies
export const TeamSchema = z.object({
  id: TeamIdSchema,
  displayName: z.string(),
})

export const ShowMyselfResponseSchema = z.object({
  user: UserSchema,
  team: TeamSchema,
})

export const TestSchema = z.object({
  id: TestIdSchema,
  name: z.string(),
  expectedOutput: z.string(),
  owner: TeamIdSchema,
})

export const TeamInfoSchema = z.object({
  team: TeamSchema,
  members: z.array(UserSchema),
})

export const TestSummarySchema = z.object({
  id: TestIdSchema,
  name: z.string(),
  creator: z.string(),
})

export type AbortedExecution = z.infer<typeof AbortedExecutionSchema>
export type ExecutionExitStatus = z.infer<typeof ExecutionExitStatusSchema>
export type ExecutionOutput = z.infer<typeof ExecutionOutputSchema>
export type FinishedCompilerTask = z.infer<typeof FinishedCompilerTaskSchema>
export type FinishedCompilerTaskSummary = z.infer<typeof FinishedCompilerTaskSummarySchema>
export type FinishedExecution = z.infer<typeof FinishedExecutionSchema>
export type FinishedTaskInfo = z.infer<typeof FinishedTaskInfoSchema>
export type FinishedTest = z.infer<typeof FinishedTestSchema>
export type FinishedTestSummary = z.infer<typeof FinishedTestSummarySchema>
export type InternalError = z.infer<typeof InternalErrorSchema>
export type Repo = z.infer<typeof RepoSchema>
export type ShowMyselfResponse = z.infer<typeof ShowMyselfResponseSchema>
export type TaskId = z.infer<typeof TaskIdSchema>
export type Team = z.infer<typeof TeamSchema>
export type TeamId = z.infer<typeof TeamIdSchema>
export type TeamInfo = z.infer<typeof TeamInfoSchema>
export type Test = z.infer<typeof TestSchema>
export type TestId = z.infer<typeof TestIdSchema>
export type TestSummary = z.infer<typeof TestSummarySchema>
export type User = z.infer<typeof UserSchema>
export type UserId = z.infer<typeof UserIdSchema>
