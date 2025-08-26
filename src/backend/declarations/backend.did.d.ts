import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface Person { 'id' : number, 'age' : number, 'name' : string }
export interface PersonInput { 'age' : number, 'name' : string }
export interface QueryParams { 'offset' : number, 'limit' : number }
export type Result = { 'Ok' : Person } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : Array<Person> } |
  { 'Err' : string };
export interface UpdateParams { 'id' : number, 'name' : string }
export interface _SERVICE {
  'person_create' : ActorMethod<[PersonInput], Result>,
  'person_delete' : ActorMethod<[number], Result>,
  'person_query' : ActorMethod<[QueryParams], Result_1>,
  'person_update' : ActorMethod<[UpdateParams], Result>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
